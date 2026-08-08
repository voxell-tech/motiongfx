use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{
    DeriveInput, Expr, Field, FieldValue, GenericParam, Token,
};

use crate::common::named_fields;

/// What `#[default(..)]` says a field starts as.
enum Start {
    /// `#[default(px(4))]`: this value, whole.
    Value(Expr),
    /// `#[default(size: 24, weight: Bold)]`: the field's own default,
    /// with these of its fields overridden.
    Overrides(Punctuated<FieldValue, Token![,]>),
}

pub fn expand(ast: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &ast.ident;
    let fields = named_fields(ast, "OverrideDefault")?;

    let mut values = Vec::new();

    for field in fields {
        let field_name =
            field.ident.as_ref().expect("named fields checked above");
        let ty = &field.ty;

        values.push(match start(field)? {
            None => quote! {
                #field_name: ::core::default::Default::default(),
            },
            Some(Start::Value(expr)) => quote!(#field_name: #expr,),
            // Annotated, so the overrides resolve against the field's
            // own type rather than waiting on inference.
            Some(Start::Overrides(overrides)) => {
                let writes = overrides.iter().map(|over| {
                    let member = &over.member;
                    let value = &over.expr;
                    quote!(value.#member = #value;)
                });

                quote! {
                    #field_name: {
                        let mut value: #ty =
                            ::core::default::Default::default();
                        #(#writes)*
                        value
                    },
                }
            }
        });
    }

    // Same as the standard derive: every parameter has to be `Default`
    // for the whole to be.
    let bounds =
        ast.generics.params.iter().filter_map(|param| match param {
            GenericParam::Type(ty) => {
                let ident = &ty.ident;
                Some(quote!(#ident: ::core::default::Default))
            }
            _ => None,
        });

    let (impl_generics, ty_generics, where_clause) =
        ast.generics.split_for_impl();

    let predicates =
        where_clause.map(|clause| &clause.predicates).into_iter();

    Ok(quote! {
        impl #impl_generics ::core::default::Default
            for #name #ty_generics
        where
            #(#predicates,)*
            #(#bounds,)*
        {
            fn default() -> Self {
                Self { #(#values)* }
            }
        }
    })
}

/// The `#[default(..)]` on this field, if it carries one.
fn start(field: &Field) -> syn::Result<Option<Start>> {
    let Some(attr) = field
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("default"))
    else {
        return Ok(None);
    };

    // `size: 24` is not an expression, so a list of them parses only
    // as overrides. A bare `size` would parse either way, and means
    // the value: hence the check for a colon on every one.
    let overrides = attr.parse_args_with(
        Punctuated::<FieldValue, Token![,]>::parse_terminated,
    );

    if let Ok(overrides) = overrides
        && !overrides.is_empty()
        && overrides.iter().all(|over| over.colon_token.is_some())
    {
        return Ok(Some(Start::Overrides(overrides)));
    }

    Ok(Some(Start::Value(attr.parse_args()?)))
}
