use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Field, parse_quote};

use crate::common::{
    crate_path, generics, named_fields, option_inner, pascal_case,
    snake_case,
};

pub fn expand(ast: &DeriveInput) -> syn::Result<TokenStream2> {
    let root = crate_path();
    let name = &ast.ident;
    let fields = named_fields(ast, "element")?;

    let generics = generics(ast)?;
    let decl = &generics.decl;
    let ty = &generics.ty;
    let bounds = &generics.where_clause;
    let predicates = &generics.predicates;

    let path_mod = format_ident!("{}_path", snake_case(name));
    let field_enum = format_ident!("{}Field", name);

    let mut variants = Vec::new();
    let mut ids = Vec::new();
    let mut lookups = Vec::new();

    let mut elem_bounds = Vec::new();
    let mut builds = Vec::new();
    let mut patches = Vec::new();
    let mut despawns = Vec::new();

    for field in fields.iter() {
        let field_name =
            field.ident.as_ref().expect("named fields checked above");
        let marker = quote!(#path_mod::#field_name #ty);
        let id = quote!(<#marker as #root::lenz::FieldPath>::id());

        // A field marked `#[elem(child)]` is an element in its own
        // right. It is absent from the enum, because naming it there
        // would offer a second, redundant way in.
        if is_child(field)? {
            // `Option<T>` builds nothing when absent, so the store
            // simply has no entry and the walk stops there.
            let (elem_ty, elem) = match option_inner(&field.ty) {
                Some(inner) => {
                    (inner, quote!(self.#field_name.as_ref()))
                }
                None => (
                    &field.ty,
                    quote!(::core::option::Option::Some(&self.#field_name)),
                ),
            };

            elem_bounds
                .push(quote!(#elem_ty: #root::element::Element<H>));

            // Fully qualified throughout, so a user of the derive
            // need not have our traits in scope.
            let as_elem =
                quote!(<#elem_ty as #root::element::Element<H>>);

            builds.push(quote! {
                if let ::core::option::Option::Some(elem) = #elem {
                    let child = #as_elem::build(
                        elem, world, node, records, theme,
                    );
                    records.store_mut().insert(node, #id, child);
                }
            });

            patches.push(quote! {
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

        // A field marked `#[elem(ignore)]` only ever changes at
        // build. Leaving it out of the enum the same way `child` does
        // makes that true rather than merely asked for: nothing can
        // name it to walk a path there, so a stray `.bind()` writes
        // the field and has no way to tell the backend. The struct is
        // re-emitted with `#[lenz(ignore)]` in its place, so the
        // cursor has no `.field()` for it either.
        if is_ignore(field)? {
            continue;
        }

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
    }

    let subject = rewrite_struct(ast, &root)?;

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

        impl<H, #decl> #root::element::Element<H> for #name #ty
        where
            H: #root::host::Host,
            Self: #root::element::ElementVisual<H>
                + ::core::marker::Send
                + ::core::marker::Sync,
            #(#elem_bounds,)*
            #predicates
        {
            fn build(
                &self,
                world: &mut H::World,
                parent: H::Node,
                records: &mut #root::records::Records<H>,
                theme: &H::Theme,
            ) -> H::Node {
                let node = <H as #root::host::Host>::spawn(
                    world, parent,
                );

                #(#builds)*

                let (lanes, store) = records.build_parts();
                let mut draw = #root::ui::Build::new(
                    world, node, lanes, store, theme,
                );
                <Self as #root::element::ElementVisual<H>>
                    ::build_fields(self, &mut draw);
                node
            }

            fn patch(
                &self,
                world: &mut H::World,
                node: H::Node,
                path: &[#root::lenz::FieldId],
                store: &mut #root::store::Store<H>,
                theme: &H::Theme,
            ) {
                let ::core::option::Option::Some((head, rest)) =
                    path.split_first()
                else {
                    return;
                };

                #(#patches)*

                // Whatever else the walk had in mind, this element
                // draws the field whole.
                let _ = rest;

                if let ::core::option::Option::Some(field) =
                    <Self as #root::element::Fields>::field(*head)
                {
                    let mut patch =
                        #root::ui::Patch::new(world, node, theme);
                    <Self as #root::element::ElementVisual<H>>
                        ::patch_fields(self, &mut patch, field);
                }
            }

            fn despawn(
                &self,
                world: &mut H::World,
                node: H::Node,
                store: &mut #root::store::Store<H>,
            ) {
                #(#despawns)*
                <H as #root::host::Host>::despawn(world, node);
            }
        }
    })
}

/// The struct itself, re-emitted for the field/patch system to derive
/// off: `#[elem(child)]` markers dropped, `#[elem(ignore)]` swapped
/// for `#[lenz(ignore)]`, and `Lenz`/`OverrideDefault` derived - the
/// dispatch below names a field by the id `Lenz` gives it, and an
/// element's own fields almost always want `#[default(...)]`.
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
        let mut kept = Vec::with_capacity(field.attrs.len());
        for attr in field.attrs.drain(..) {
            if !attr.path().is_ident("elem") {
                kept.push(attr);
                continue;
            }
            match attr.parse_args::<syn::Ident>() {
                Ok(directive) if directive == "child" => {}
                Ok(directive) if directive == "ignore" => {
                    kept.push(parse_quote!(#[lenz(ignore)]));
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        attr,
                        "expected `#[elem(child)]` or \
                         `#[elem(ignore)]`",
                    ));
                }
            }
        }
        field.attrs = kept;
    }

    Ok(out)
}

/// What `#[elem(...)]` says about this field: `child` for one that is
/// an element in its own right, `ignore` for one that only ever
/// changes at build.
fn elem_directive(field: &Field) -> syn::Result<Option<syn::Ident>> {
    let Some(attr) =
        field.attrs.iter().find(|attr| attr.path().is_ident("elem"))
    else {
        return Ok(None);
    };
    attr.parse_args::<syn::Ident>().map(Some)
}

fn is_child(field: &Field) -> syn::Result<bool> {
    Ok(elem_directive(field)?
        .is_some_and(|directive| directive == "child"))
}

fn is_ignore(field: &Field) -> syn::Result<bool> {
    Ok(elem_directive(field)?
        .is_some_and(|directive| directive == "ignore"))
}
