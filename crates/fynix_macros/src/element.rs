use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Expr, Field, Fields, Ident, LitInt, Path, Token,
    parse_quote, punctuated::Punctuated,
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
    default: Option<FieldDefault>,
    anim: Option<AnimConfig>,
}

/// `#[elem(anim(ms = .., ease = .., lerp = .., on(..), ..))]`: how a
/// field travels, and the tags it answers to.
struct AnimConfig {
    ms: u32,
    ease: Option<Expr>,
    /// Defaults to `<T as Interpolation<()>>::interp`.
    lerp: Option<Expr>,
    /// In priority order, first match wins.
    lines: Vec<AnimLine>,
}

/// One `on(<tag>, value = <field>, ms = .., ease = ..)`.
struct AnimLine {
    /// A value of the tag type: a unit struct, or an enum variant.
    tag: Expr,
    /// The sibling field the destination value is read from.
    value: Ident,
    ms: Option<u32>,
    ease: Option<Expr>,
}

/// `Tween::ms(ms, lerp).ease(ease)` for one destination.
#[allow(clippy::too_many_arguments)]
fn tween_of(
    root: &TokenStream2,
    field_ty: &syn::Type,
    ms: u32,
    ease: Option<&Expr>,
    lerp: Option<&Expr>,
) -> TokenStream2 {
    let lerp = match lerp {
        Some(lerp) => quote!(#lerp),
        None => quote! {
            <#field_ty as #root::tween::Interpolation<()>>::interp
        },
    };
    let tween = quote!(#root::tween::Tween::ms(#ms, #lerp));
    match ease {
        Some(ease) => quote!(#tween.ease(#ease)),
        None => tween,
    }
}

/// One `registrar.field(...)` call: the field's base, then its lines
/// in the order they were written.
#[allow(clippy::too_many_arguments)]
fn anim_field(
    root: &TokenStream2,
    host: &Path,
    name: &Ident,
    ty: &TokenStream2,
    id: &TokenStream2,
    patch: &Path,
    field_name: &Ident,
    field_ty: &syn::Type,
    anim: &AnimConfig,
) -> TokenStream2 {
    let base_tween = tween_of(
        root,
        field_ty,
        anim.ms,
        anim.ease.as_ref(),
        anim.lerp.as_ref(),
    );

    let lines = anim.lines.iter().map(|line| {
        let tag = &line.tag;
        let value = &line.value;
        let tween = tween_of(
            root,
            field_ty,
            line.ms.unwrap_or(anim.ms),
            line.ease.as_ref().or(anim.ease.as_ref()),
            anim.lerp.as_ref(),
        );
        quote! {
            .on(
                |__elements, __node| #root::anim::tagged::<#host, _>(
                    __elements, __node, #tag,
                ),
                |__elements, __node| __elements
                    .get::<#name #ty>(&__node)
                    .map(|__element| &__element.#value),
                #tween,
            )
        }
    });

    quote! {
        __registrar.field(
            #id,
            <#patch as #root::ui::FieldPatch<#host>>::patch,
            |__elements, __node| __elements
                .get::<#name #ty>(&__node)
                .map(|__element| &__element.#field_name),
            #base_tween,
            |__lines| {
                __lines #(#lines)*;
            },
        );
    }
}

fn parse_anim(meta: &syn::meta::ParseNestedMeta) -> syn::Result<AnimConfig> {
    let mut ms = None;
    let mut ease = None;
    let mut lerp = None;
    let mut lines = Vec::new();

    meta.parse_nested_meta(|item| {
        if item.path.is_ident("ms") {
            ms = Some(item.value()?.parse::<LitInt>()?.base10_parse()?);
        } else if item.path.is_ident("ease") {
            ease = Some(item.value()?.parse()?);
        } else if item.path.is_ident("lerp") {
            lerp = Some(item.value()?.parse()?);
        } else if item.path.is_ident("on") {
            lines.push(parse_line(&item)?);
        } else {
            return Err(item.error(
                "expected `ms`, `ease`, `lerp`, or `on(...)`",
            ));
        }
        Ok(())
    })?;

    let Some(ms) = ms else {
        return Err(meta.error("`anim` needs `ms = ...`"));
    };

    Ok(AnimConfig {
        ms,
        ease,
        lerp,
        lines,
    })
}

fn parse_line(
    meta: &syn::meta::ParseNestedMeta,
) -> syn::Result<AnimLine> {
    let mut tag: Option<Expr> = None;
    let mut value: Option<Ident> = None;
    let mut ms = None;
    let mut ease = None;

    meta.parse_nested_meta(|item| {
        if item.path.is_ident("value") {
            value = Some(item.value()?.parse()?);
        } else if item.path.is_ident("ms") {
            ms = Some(item.value()?.parse::<LitInt>()?.base10_parse()?);
        } else if item.path.is_ident("ease") {
            ease = Some(item.value()?.parse()?);
        } else if tag.is_none() {
            // The bare leading path is the tag itself, read as a
            // value so `Pressed` and `ToggleState::On` are alike.
            let path = item.path.clone();
            tag = Some(parse_quote!(#path));
        } else {
            return Err(
                item.error("expected `value`, `ms`, or `ease`")
            );
        }
        Ok(())
    })?;

    let Some(tag) = tag else {
        return Err(meta.error("`on` needs a tag"));
    };
    let Some(value) = value else {
        return Err(meta.error("`on` needs `value = <field>`"));
    };

    Ok(AnimLine {
        tag,
        value,
        ms,
        ease,
    })
}

/// The value a field starts from in `ElementBase::base`.
enum FieldDefault {
    /// `#[elem(default = <expr>)]`, whole.
    Expr(Expr),
    /// `#[elem(default = ::<expr>)]`, resolved against the field's own
    /// type: `::NONE` on a `Color` field is `<Color>::NONE`.
    Relative(Expr),
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
                let value = meta.value()?;
                cfg.default = Some(if value.peek(Token![::]) {
                    value.parse::<Token![::]>()?;
                    FieldDefault::Relative(value.parse()?)
                } else {
                    FieldDefault::Expr(value.parse()?)
                });
            } else if meta.path.is_ident("anim") {
                cfg.anim = Some(parse_anim(&meta)?);
            } else {
                return Err(meta.error(
                    "expected `child`, `ignore`, `patch = ...`, \
                     `default = ...`, or `anim(...)`",
                ));
            }
            Ok(())
        })?;
    }

    if cfg.child && (cfg.ignore || cfg.patch.is_some()) {
        return Err(syn::Error::new_spanned(
            field,
            "`#[elem(child)]` takes only `default = ...`",
        ));
    }
    if cfg.ignore && cfg.patch.is_some() {
        return Err(syn::Error::new_spanned(
            field,
            "`#[elem(ignore)]` and `#[elem(patch)]` are exclusive",
        ));
    }
    if cfg.anim.is_some() && cfg.patch.is_none() {
        return Err(syn::Error::new_spanned(
            field,
            "`#[elem(anim(...))]` needs `patch = ...` to write \
             through",
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
    let mut field_inits = Vec::new();
    let mut base_bounds = Vec::new();
    let mut anim_fields = Vec::new();

    for field in fields.iter() {
        let field_name =
            field.ident.as_ref().expect("named fields checked above");
        let cfg = field_config(field)?;

        let marker = quote!(#path_mod::#field_name #ty);
        let id = quote!(<#marker as #root::lenz::FieldPath>::id());

        let field_ty = &field.ty;
        let init = match &cfg.default {
            Some(FieldDefault::Expr(expr)) => quote!(#expr),
            Some(FieldDefault::Relative(expr)) => {
                quote!(<#field_ty>::#expr)
            }
            None if cfg.child => match option_inner(field_ty) {
                Some(_) => quote!(::core::option::Option::None),
                None => {
                    base_bounds.push(quote!(
                        #field_ty: #root::element::ElementBase<#host>
                    ));
                    quote! {
                        <#field_ty as #root::element::ElementBase<#host>>::base(
                            theme,
                        )
                    }
                }
            },
            None => {
                base_bounds.push(
                    quote!(#field_ty: ::core::default::Default),
                );
                quote!(::core::default::Default::default())
            }
        };
        field_inits.push(quote!(#field_name: #init,));

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

        if let Some(anim) = &cfg.anim {
            anim_fields.push(anim_field(
                &root, host, name, ty, &id, patch, field_name,
                field_ty, anim,
            ));
        }
    }

    let subject = rewrite_struct(ast, &root)?;

    let build_hook = opts.build.map(|build_fn| {
        quote! {
            {
                let (__transitions, __store) = records.build_parts();
                let mut __draw = #root::ui::Build::new(
                    world, node, __transitions, __store, theme,
                );
                (#build_fn)(self, &mut __draw);
            }
        }
    });

    // Guarded inside, so only the first build of this type pays.
    let anim_register = (!anim_fields.is_empty()).then(|| {
        quote! {
            records.register_anim(
                ::core::any::TypeId::of::<Self>(),
                |__registrar| { #(#anim_fields)* },
            );
        }
    });

    let ctor = match &ast.data {
        Data::Struct(data) if matches!(data.fields, Fields::Unit) => {
            quote!(Self)
        }
        _ => quote!(Self { #(#field_inits)* }),
    };

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
            #(#base_bounds,)*
            #predicates
        {
            fn base(theme: &#host_theme) -> Self {
                let _ = theme;
                #ctor
            }
        }

        impl<#decl> #root::style::Seed<#host_theme> for #name #ty
        where
            #(#base_bounds,)*
            #predicates
        {
            fn seed(theme: &#host_theme) -> Self {
                <Self as #root::element::ElementBase<#host>>::base(
                    theme,
                )
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

                #anim_register

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
/// `patch = X` becomes `tag = X`), with `Lenz` derived.
fn rewrite_struct(
    ast: &DeriveInput,
    root: &TokenStream2,
) -> syn::Result<DeriveInput> {
    let mut out = ast.clone();

    out.attrs.push(parse_quote! {
        #[derive(#root::lenz::Lenz)]
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
