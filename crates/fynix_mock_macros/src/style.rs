use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{FnArg, ItemFn, Pat, PatType, Type, Visibility};

use crate::common::{crate_path, pascal_case};

pub fn expand(item: &ItemFn) -> syn::Result<TokenStream2> {
    let root = crate_path();
    let vis = &item.vis;
    let name = format_ident!("{}", pascal_case(&item.sig.ident));

    if !item.sig.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.sig.generics,
            "a style is a type, so it takes its arguments as fields \
             rather than parameters",
        ));
    }

    let Some(first) = item.sig.inputs.first() else {
        return Err(syn::Error::new_spanned(
            &item.sig,
            "the first argument is the element this styles, as \
             `&mut Element`",
        ));
    };

    let target = element_type(first)?;
    let element = binding(first)?;

    // Everything after the element is what the style carries.
    let carried = item.sig.inputs.iter().skip(1);

    let fields = carried
        .clone()
        .map(|arg| field(arg, vis))
        .collect::<syn::Result<Vec<_>>>()?;

    let names =
        carried.map(binding).collect::<syn::Result<Vec<_>>>()?;

    // What the fields are called inside the body, and they are named
    // by the caller, so nothing the macro writes has to be visible.
    let unpack = if names.is_empty() {
        quote!()
    } else {
        quote!(let Self { #(#names,)* } = self;)
    };

    let declare = if fields.is_empty() {
        quote!(#vis struct #name;)
    } else {
        quote!(#vis struct #name { #(#fields,)* })
    };

    let attrs = &item.attrs;
    let body = &item.block;

    Ok(quote! {
        #(#attrs)*
        #declare

        impl #root::style::Style for #name {
            type Element = #target;

            fn apply(self, #element: &mut Self::Element) {
                #unpack
                #body
            }
        }
    })
}

/// The `T` of the `&mut T` an argument takes.
fn element_type(arg: &FnArg) -> syn::Result<&Type> {
    let FnArg::Typed(PatType { ty, .. }) = arg else {
        return Err(syn::Error::new_spanned(
            arg,
            "a style is applied to an element, not to itself",
        ));
    };

    let Type::Reference(reference) = &**ty else {
        return Err(syn::Error::new_spanned(
            ty,
            "the element is taken as `&mut Element`, to write to",
        ));
    };

    if reference.mutability.is_none() {
        return Err(syn::Error::new_spanned(
            ty,
            "the element is taken as `&mut Element`, to write to",
        ));
    }

    Ok(&reference.elem)
}

/// The name an argument binds, which the body writes through.
fn binding(arg: &FnArg) -> syn::Result<&syn::Ident> {
    let FnArg::Typed(PatType { pat, .. }) = arg else {
        return Err(syn::Error::new_spanned(
            arg,
            "a style is applied to an element, not to itself",
        ));
    };

    let Pat::Ident(ident) = &**pat else {
        return Err(syn::Error::new_spanned(
            pat,
            "each argument is one name, which becomes a field",
        ));
    };

    Ok(&ident.ident)
}

/// An argument, as the field it becomes.
fn field(arg: &FnArg, vis: &Visibility) -> syn::Result<TokenStream2> {
    let name = binding(arg)?;

    let FnArg::Typed(PatType { ty, .. }) = arg else {
        return Err(syn::Error::new(
            arg.span(),
            "expected an argument",
        ));
    };

    Ok(quote!(#vis #name: #ty))
}
