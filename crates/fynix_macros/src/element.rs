use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Expr, Field, Path, parse_quote,
    punctuated::Punctuated,
};

use crate::common::{
    crate_path, generics, named_fields, option_inner, pascal_case,
    snake_case,
};

/// What `#[element(...)]` carries: the backend to build against and an
/// optional structural hook.
///
/// `build` holds whatever expression `#[element(build = ...)]` names -
/// a free `fn(&Self, &mut Build)`, or `Self::build` for the element's
/// own inherent method.
pub struct ElementArgs {
    pub host: Path,
    pub build: Option<Expr>,
}

impl ElementArgs {
    pub fn parse(args: TokenStream2) -> syn::Result<Self> {
        let mut host: Option<Path> = None;
        let mut build: Option<Expr> = None;

        if !args.is_empty() {
            let parser = syn::meta::parser(|meta| {
                if meta.path.is_ident("host") {
                    host = Some(meta.value()?.parse()?);
                    Ok(())
                } else if meta.path.is_ident("build") {
                    build = Some(meta.value()?.parse()?);
                    Ok(())
                } else {
                    Err(meta.error("expected `host` or `build`"))
                }
            });
            syn::parse::Parser::parse2(parser, args)?;
        }

        Ok(Self {
            host: host
                .unwrap_or_else(|| parse_quote!(crate::FynixHost)),
            build,
        })
    }
}

/// What `#[elem(...)]` says about one field.
#[derive(Default)]
struct FieldConfig {
    child: bool,
    ignore: bool,
    patch: Option<Path>,
    default: Option<Expr>,
}

fn field_config(field: &Field) -> syn::Result<FieldConfig> {
    let mut cfg = FieldConfig::default();

    for attr in &field.attrs {
        if !attr.path().is_ident("elem") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("child") {
                cfg.child = true;
            } else if meta.path.is_ident("ignore") {
                cfg.ignore = true;
            } else if meta.path.is_ident("patch") {
                cfg.patch = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("default") {
                cfg.default = Some(meta.value()?.parse()?);
            } else {
                return Err(meta.error(
                    "expected `child`, `ignore`, `patch = ...`, or \
                     `default = ...`",
                ));
            }
            Ok(())
        })?;
    }

    if cfg.child && (cfg.ignore || cfg.patch.is_some()) {
        return Err(syn::Error::new_spanned(
            field,
            "`#[elem(child)]` takes no other directive",
        ));
    }
    if cfg.ignore && cfg.patch.is_some() {
        return Err(syn::Error::new_spanned(
            field,
            "`#[elem(ignore)]` and `#[elem(patch)]` are exclusive",
        ));
    }

    Ok(cfg)
}

pub fn expand(
    args: TokenStream2,
    ast: &DeriveInput,
) -> syn::Result<TokenStream2> {
    let root = crate_path();
    let opts = ElementArgs::parse(args)?;
    let host = &opts.host;

    let name = &ast.ident;
    let fields = named_fields(ast, "element")?;

    let generics = generics(ast)?;
    let decl = &generics.decl;
    let ty = &generics.ty;
    let bounds = &generics.where_clause;
    let predicates = &generics.predicates;

    let path_mod = format_ident!("{}_path", snake_case(name));
    let field_enum = format_ident!("{}Field", name);

    let host_node = quote!(<#host as #root::host::Host>::Node);
    let host_world = quote!(<#host as #root::host::Host>::World);
    let host_theme = quote!(<#host as #root::host::Host>::Theme);

    let mut variants = Vec::new();
    let mut ids = Vec::new();
    let mut lookups = Vec::new();

    let mut elem_bounds = Vec::new();
    let mut builds = Vec::new();
    let mut child_patches = Vec::new();
    let mut own_patches = Vec::new();
    let mut field_writes = Vec::new();
    let mut despawns = Vec::new();
    let mut default_overrides = Vec::new();

    for field in fields.iter() {
        let field_name =
            field.ident.as_ref().expect("named fields checked above");
        let cfg = field_config(field)?;

        let marker = quote!(#path_mod::#field_name #ty);
        let id = quote!(<#marker as #root::lenz::FieldPath>::id());

        if let Some(default) = &cfg.default {
            default_overrides
                .push(quote!(__elem.#field_name = #default;));
        }

        // A `#[elem(child)]` field is an element in its own right. It
        // is absent from the enum: naming it there would offer a
        // second, redundant way in.
        if cfg.child {
            let (elem_ty, elem) = match option_inner(&field.ty) {
                Some(inner) => {
                    (inner, quote!(self.#field_name.as_ref()))
                }
                None => (
                    &field.ty,
                    quote!(::core::option::Option::Some(&self.#field_name)),
                ),
            };

            elem_bounds.push(
                quote!(#elem_ty: #root::element::Element<#host>),
            );

            let as_elem = quote!(
                <#elem_ty as #root::element::Element<#host>>
            );

            builds.push(quote! {
                if let ::core::option::Option::Some(elem) = #elem {
                    let child = #as_elem::build(
                        elem, world, node, records, theme,
                    );
                    records.store_mut().insert(node, #id, child);
                }
            });

            child_patches.push(quote! {
                if *head == #id {
                    if let (
                        ::core::option::Option::Some(elem),
                        ::core::option::Option::Some(child),
                    ) = (#elem, store.get(node, *head))
                    {
                        #as_elem::patch(
                            elem, world, child, rest, store, theme,
                        );
                    }
                    return;
                }
            });

            despawns.push(quote! {
                if let (
                    ::core::option::Option::Some(elem),
                    ::core::option::Option::Some(child),
                ) = (#elem, store.take(node, #id))
                {
                    #as_elem::despawn(elem, world, child, store);
                }
            });

            continue;
        }

        // A `#[elem(ignore)]` field only ever changes at build. It is
        // left out of the enum and, through `#[lenz(ignore)]` on the
        // re-emitted struct, out of the cursor: nothing can name a
        // path there.
        if cfg.ignore {
            continue;
        }

        let Some(patch) = &cfg.patch else {
            return Err(syn::Error::new_spanned(
                field,
                "an own field needs `#[elem(patch = ...)]` or \
                 `#[elem(ignore)]`",
            ));
        };

        let variant = format_ident!("{}", pascal_case(field_name));
        variants.push(quote!(#variant,));
        ids.push(quote!(#field_enum::#variant => #id,));
        lookups.push(quote! {
            if id == #id {
                return ::core::option::Option::Some(
                    #field_enum::#variant,
                );
            }
        });

        field_writes.push(quote! {
            {
                let mut __patch = #root::ui::Patch::new(
                    world, node, theme,
                );
                <#patch as #root::ui::FieldPatch<#host>>::patch(
                    &mut __patch, &self.#field_name,
                );
            }
        });

        own_patches.push(quote! {
            if *head == #id {
                let mut __patch = #root::ui::Patch::new(
                    world, node, theme,
                );
                <#patch as #root::ui::FieldPatch<#host>>::patch(
                    &mut __patch, &self.#field_name,
                );
                return;
            }
        });
    }

    let subject = rewrite_struct(ast, &root)?;

    let build_hook = opts.build.map(|build_fn| {
        quote! {
            {
                let (__tweens, __store) = records.build_parts();
                let mut __draw = #root::ui::Build::new(
                    world, node, __tweens, __store, theme,
                );
                (#build_fn)(self, &mut __draw);
            }
        }
    });

    Ok(quote! {
        #subject

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum #field_enum {
            #(#variants)*
        }

        impl<#decl> #root::element::Fields for #name #ty #bounds {
            type Field = #field_enum;

            fn field(
                id: #root::lenz::FieldId,
            ) -> ::core::option::Option<#field_enum> {
                #(#lookups)*
                ::core::option::Option::None
            }

            fn field_id(field: #field_enum) -> #root::lenz::FieldId {
                match field {
                    #(#ids)*
                }
            }
        }

        impl<#decl> #root::element::ElementBase<#host> for #name #ty
        where
            Self: ::core::default::Default,
            #predicates
        {
            fn base(theme: &#host_theme) -> Self {
                let _ = theme;
                let mut __elem =
                    <Self as ::core::default::Default>::default();
                #(#default_overrides)*
                __elem
            }
        }

        impl<#decl> #root::element::Element<#host> for #name #ty
        where
            Self: ::core::marker::Send + ::core::marker::Sync,
            #(#elem_bounds,)*
            #predicates
        {
            fn build(
                &self,
                world: &mut #host_world,
                parent: #host_node,
                records: &mut #root::records::Records<#host>,
                theme: &#host_theme,
            ) -> #host_node {
                let node =
                    <#host as #root::host::Host>::spawn(world, parent);

                #(#builds)*

                #build_hook

                #(#field_writes)*

                node
            }

            fn patch(
                &self,
                world: &mut #host_world,
                node: #host_node,
                path: &[#root::lenz::FieldId],
                store: &mut #root::store::Store<#host>,
                theme: &#host_theme,
            ) {
                let ::core::option::Option::Some((head, rest)) =
                    path.split_first()
                else {
                    return;
                };

                #(#child_patches)*

                // An own field is drawn whole, whatever else the walk
                // had in mind.
                let _ = rest;

                #(#own_patches)*
            }

            fn despawn(
                &self,
                world: &mut #host_world,
                node: #host_node,
                store: &mut #root::store::Store<#host>,
            ) {
                #(#despawns)*
                <#host as #root::host::Host>::despawn(world, node);
            }
        }
    })
}

/// The struct, re-emitted for the path system to derive off.
/// `#[elem(...)]` markers become `#[lenz(...)]` ones (`ignore` stays,
/// `patch = X` becomes `tag = X`), with `Lenz` / `OverrideDefault`
/// derived.
fn rewrite_struct(
    ast: &DeriveInput,
    root: &TokenStream2,
) -> syn::Result<DeriveInput> {
    let mut out = ast.clone();

    out.attrs.push(parse_quote! {
        #[derive(#root::lenz::Lenz, #root::OverrideDefault)]
    });
    out.attrs.push(parse_quote!(#[lenz(crate = #root::lenz)]));

    let Data::Struct(data) = &mut out.data else {
        return Err(syn::Error::new_spanned(
            ast,
            "`#[element]` only applies to structs",
        ));
    };

    for field in &mut data.fields {
        let cfg = field_config(field)?;
        let mut kept: Punctuated<syn::Attribute, syn::Token![,]> =
            Punctuated::new();
        for attr in field.attrs.drain(..) {
            if !attr.path().is_ident("elem") {
                kept.push(attr);
            }
        }
        field.attrs = kept.into_iter().collect();
        if cfg.ignore {
            field.attrs.push(parse_quote!(#[lenz(ignore)]));
        } else if let Some(patch) = &cfg.patch {
            field.attrs.push(parse_quote!(#[lenz(tag = #patch)]));
        }
    }

    Ok(out)
}
