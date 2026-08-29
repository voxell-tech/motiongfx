use std::borrow::Cow;

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{
    Attribute, Data, DeriveInput, Fields, GenericArgument,
    GenericParam, Ident, PathArguments, Type, WhereClause,
};

/// A struct's generics, in the pieces the derive splices.
pub struct Generics<'a> {
    /// The parameters with their bounds: `S: Look, T`. Empty when the
    /// struct has none, and safe to follow a parameter of our own, as
    /// `impl<P, #decl>`.
    pub decl: TokenStream2,
    /// Just the names: `S, T`.
    pub idents: Vec<&'a Ident>,
    /// The arguments as written on the type: `<S, T>`, or nothing.
    pub ty: TokenStream2,
    /// The struct's own `where`, plus `'static` for every parameter.
    ///
    /// A path is named by its `TypeId`, so nothing along it may
    /// borrow.
    pub where_clause: TokenStream2,
    /// The same predicates without the `where`, comma terminated, for
    /// an impl that adds predicates of its own.
    pub predicates: TokenStream2,
}

/// Reads a struct's generics, rejecting what a path cannot carry.
pub fn generics(ast: &DeriveInput) -> syn::Result<Generics<'_>> {
    let mut idents = Vec::new();

    for param in &ast.generics.params {
        match param {
            GenericParam::Type(ty) => idents.push(&ty.ident),
            GenericParam::Lifetime(param) => {
                return Err(syn::Error::new_spanned(
                    param,
                    "a field path is named by its `TypeId`, which a \
                     borrow cannot have",
                ));
            }
            GenericParam::Const(param) => {
                return Err(syn::Error::new_spanned(
                    param,
                    "const generics are not supported yet",
                ));
            }
        }
    }

    let decl = if ast.generics.params.is_empty() {
        TokenStream2::new()
    } else {
        let params = &ast.generics.params;
        quote!(#params)
    };

    let ty = if idents.is_empty() {
        TokenStream2::new()
    } else {
        quote!(<#(#idents),*>)
    };

    let predicates =
        predicates(ast.generics.where_clause.as_ref(), &idents);

    let where_clause = if predicates.is_empty() {
        TokenStream2::new()
    } else {
        quote!(where #predicates)
    };

    Ok(Generics {
        decl,
        idents,
        ty,
        where_clause,
        predicates,
    })
}

/// The struct's predicates, plus `'static` for every parameter.
fn predicates(
    existing: Option<&WhereClause>,
    idents: &[&Ident],
) -> TokenStream2 {
    let existing =
        existing.map(|clause| &clause.predicates).into_iter();
    quote! {
        #(#existing,)*
        #(#idents: 'static,)*
    }
}

/// A struct's named fields, of which a unit struct has none.
pub fn named_fields(
    ast: &DeriveInput,
) -> syn::Result<Cow<'_, Punctuated<syn::Field, syn::Token![,]>>> {
    match &ast.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => Ok(Cow::Borrowed(&named.named)),
            Fields::Unit => Ok(Cow::Owned(Punctuated::new())),
            Fields::Unnamed(_) => Err(syn::Error::new_spanned(
                &data.fields,
                "`#[derive(Lenz)]` needs named fields, or none at all",
            )),
        },
        _ => Err(syn::Error::new(
            Span::call_site(),
            "`#[derive(Lenz)]` only applies to structs",
        )),
    }
}

/// The `T` of an `Option<T>`, if that is what this type is.
pub fn option_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    if path.qself.is_some() {
        return None;
    }

    let segment = path.path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }

    let PathArguments::AngleBracketed(args) = &segment.arguments
    else {
        return None;
    };

    args.args.iter().find_map(|arg| match arg {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    })
}

/// Where the generated code reaches the `lenz` crate.
///
/// `#[lenz(crate = <path>)]` on the struct wins. Otherwise the
/// dependency's own name, following a rename. `Itself` and a missing
/// dependency both fall back to `::lenz`, which `extern crate self as
/// lenz` keeps valid inside the crate and its tests.
pub fn lenz_root(attrs: &[Attribute]) -> syn::Result<TokenStream2> {
    for attr in attrs {
        if !attr.path().is_ident("lenz") {
            continue;
        }
        let mut path = None;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("crate") {
                path = Some(meta.value()?.parse::<syn::Path>()?);
                Ok(())
            } else {
                // `ignore` is a field directive; anything else is a
                // typo. Neither belongs on the struct's `crate`
                // lookup, but only `crate` is read here.
                Err(meta.error("expected `crate = <path>`"))
            }
        })?;
        if let Some(path) = path {
            return Ok(quote!(#path));
        }
    }

    Ok(match crate_name("lenz") {
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(::#ident)
        }
        Ok(FoundCrate::Itself) | Err(_) => quote!(::lenz),
    })
}

/// Whether the field carries `#[lenz(ignore)]`.
pub fn ignored(field: &syn::Field) -> bool {
    field.attrs.iter().any(|attr| {
        attr.path().is_ident("lenz")
            && attr
                .parse_args::<Ident>()
                .is_ok_and(|ident| ident == "ignore")
    })
}

pub fn snake_case(ident: &Ident) -> String {
    let name = ident.to_string();
    let mut out = String::with_capacity(name.len() + 4);

    for (index, ch) in name.char_indices() {
        if ch.is_uppercase() {
            if index != 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }

    out
}
