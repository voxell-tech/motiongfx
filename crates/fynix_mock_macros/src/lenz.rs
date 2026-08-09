use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::DeriveInput;

use crate::common::{
    generics, lenz_path, named_fields, option_inner, snake_case,
};

pub fn expand(ast: &DeriveInput) -> syn::Result<TokenStream2> {
    let lenz = lenz_path();
    let name = &ast.ident;
    let fields = named_fields(ast, "Lenz")?;

    let generics = generics(ast)?;
    let decl = &generics.decl;
    let idents = &generics.idents;
    let ty = &generics.ty;
    let bounds = &generics.where_clause;
    let predicates = &generics.predicates;

    // The struct this walk starts from, with its arguments.
    let source = quote!(#name #ty);

    let path_mod = format_ident!("{}_path", snake_case(name));
    let cursor_trait = format_ident!("{}Cursor", name);

    let mut markers = Vec::new();
    let mut impls = Vec::new();
    let mut signatures = Vec::new();
    let mut bodies = Vec::new();

    for field in fields {
        let field_name =
            field.ident.as_ref().expect("named fields checked above");

        // An `Option<T>` field targets `T`, so the chain short
        // circuits instead of stopping at the `Option`.
        let (target, get, get_mut) = match option_inner(&field.ty) {
            Some(inner) => (
                inner.clone(),
                quote!(source.#field_name.as_ref()),
                quote!(source.#field_name.as_mut()),
            ),
            None => {
                let ty = &field.ty;
                (
                    ty.clone(),
                    quote!(::core::option::Option::Some(&source.#field_name)),
                    quote!(::core::option::Option::Some(&mut source.#field_name)),
                )
            }
        };

        // A marker carries the struct's parameters, because what it
        // points at may be one of them. Its bounds are left to the
        // impl below, so the declaration names nothing the module
        // would have to import.
        let declare = if idents.is_empty() {
            quote!(pub struct #field_name;)
        } else {
            quote! {
                pub struct #field_name<#(#idents),*>(
                    ::core::marker::PhantomData<
                        fn() -> (#(#idents,)*)
                    >,
                );
            }
        };

        markers.push(declare);

        // Outside the module, where the struct and its field types
        // are already in scope: nothing has to be reached through
        // `super`, which a module declared in a function body cannot
        // do.
        impls.push(quote! {
            impl<#decl> #lenz::FieldPath
                for #path_mod::#field_name #ty
            #bounds
            {
                type Source = #source;
                type Target = #target;

                #[inline(always)]
                fn get(
                    source: &#source,
                ) -> ::core::option::Option<&#target> {
                    #get
                }

                #[inline(always)]
                fn get_mut(
                    source: &mut #source,
                ) -> ::core::option::Option<&mut #target> {
                    #get_mut
                }
            }
        });

        let step =
            quote!(#lenz::Chain<P, #path_mod::#field_name #ty>);

        // Leaves hand back a cursor too; they simply have nothing
        // further to walk to.
        signatures.push(quote! {
            fn #field_name(self) -> #lenz::Cursor<#step>;
        });
        bodies.push(quote! {
            fn #field_name(self) -> #lenz::Cursor<#step> {
                #lenz::Cursor::new()
            }
        });
    }

    Ok(quote! {
        #[allow(non_camel_case_types)]
        pub mod #path_mod {
            #(#markers)*
        }

        #(#impls)*

        // The struct's parameters ride along, so that the impl below
        // constrains every one of them.
        pub trait #cursor_trait<P, #decl> {
            #(#signatures)*
        }

        impl<P, #decl> #cursor_trait<P, #(#idents,)*>
            for #lenz::Cursor<P>
        where
            P: #lenz::FieldPath<Target = #source>,
            #predicates
        {
            #(#bodies)*
        }

        impl<#decl> #name #ty #bounds {
            /// A cursor standing at this struct, to walk from.
            pub fn cursor(
            ) -> #lenz::Cursor<#lenz::Identity<Self>> {
                #lenz::Cursor::new()
            }
        }
    })
}
