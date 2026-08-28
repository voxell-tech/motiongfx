use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Data, DataEnum, DeriveInput, Expr, Field, FieldValue, Fields,
    GenericParam, PathArguments, Token, Type, Variant,
};

use crate::common::option_inner;

/// What `#[default(..)]` says a field starts as.
enum Start {
    /// `#[default(px(4))]`: this value, whole.
    Value(Expr),
    /// `#[default(::Bold)]`: a path relative to the field's own type,
    /// so a variant never has to name the enum it belongs to.
    Relative(TokenStream2),
    /// `#[default(size: 24, weight: Bold)]`: the field's own default,
    /// with these of its fields overridden. `0: 1, 1: 2` for a tuple.
    Overrides(Punctuated<FieldValue, Token![,]>),
    /// `#[default(_, 4, ..)]`: the same, written as the pattern it
    /// looks like. `_` and `..` keep what the default had.
    Positional(Vec<Slot>),
    /// `#[default(..)]`: an `Option` field holding its inner default,
    /// rather than nothing.
    Present,
}

/// One position of a pattern like `_, 4, ..`.
enum Slot {
    /// `_`: keep what the default had here.
    Wild,
    /// `..`: keep however many positions are left.
    Rest(Token![..]),
    /// Anything else: what this position becomes.
    Value(Expr),
}

impl Parse for Slot {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Before the expression, because `..` is one: a range over
        // everything.
        if input.peek(Token![..]) {
            return Ok(Self::Rest(input.parse()?));
        }

        if input.peek(Token![_]) {
            input.parse::<Token![_]>()?;
            return Ok(Self::Wild);
        }

        Ok(Self::Value(input.parse()?))
    }
}

pub fn expand(ast: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &ast.ident;

    let body = match &ast.data {
        Data::Struct(data) => build(&data.fields, quote!(Self))?,
        Data::Enum(data) => variant(data)?,
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                name,
                "`#[derive(OverrideDefault)]` does not apply to \
                 unions",
            ));
        }
    };

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
                #body
            }
        }
    })
}

/// The variant an enum starts in, marked `#[default]` the way the
/// standard derive marks it.
fn variant(data: &DataEnum) -> syn::Result<TokenStream2> {
    let mut marked = data.variants.iter().filter(|variant| {
        variant
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("default"))
    });

    let Some(chosen) = marked.next() else {
        return Err(syn::Error::new_spanned(
            &data.variants,
            "an enum needs `#[default]` on the variant it starts in",
        ));
    };

    if let Some(second) = marked.next() {
        return Err(syn::Error::new_spanned(
            second,
            "only one variant can be the default",
        ));
    }

    // The marker takes no arguments; a value belongs on the fields.
    if let Some(attr) = chosen
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("default"))
        && !matches!(attr.meta, syn::Meta::Path(_))
    {
        return Err(syn::Error::new_spanned(
            attr,
            "`#[default]` marks the variant; what it holds goes on \
             its fields",
        ));
    }

    let Variant { ident, fields, .. } = chosen;
    build(fields, quote!(Self::#ident))
}

/// `Self { .. }`, `Self(..)` or `Self`, whichever the fields are.
fn build(
    fields: &Fields,
    path: TokenStream2,
) -> syn::Result<TokenStream2> {
    match fields {
        Fields::Named(named) => {
            let values = named
                .named
                .iter()
                .map(|field| {
                    let name = &field.ident;
                    let value = value(field)?;
                    Ok(quote!(#name: #value,))
                })
                .collect::<syn::Result<Vec<_>>>()?;

            Ok(quote!(#path { #(#values)* }))
        }
        Fields::Unnamed(unnamed) => {
            let values = unnamed
                .unnamed
                .iter()
                .map(value)
                .collect::<syn::Result<Vec<_>>>()?;

            Ok(quote!(#path( #(#values,)* )))
        }
        Fields::Unit => Ok(path),
    }
}

/// What one field starts as.
fn value(field: &Field) -> syn::Result<TokenStream2> {
    let ty = &field.ty;

    Ok(match start(field)? {
        None => quote!(::core::default::Default::default()),
        Some(Start::Value(expr)) => quote!(#expr),
        // Through an alias, because a qualified path cannot carry a
        // struct variant's braces.
        Some(Start::Relative(path)) => quote! {{
            type __Default = #ty;
            __Default #path
        }},
        Some(Start::Present) => {
            let Some(inner) = option_inner(ty) else {
                return Err(syn::Error::new(
                    ty.span(),
                    "`#[default(..)]` fills an `Option`, and this \
                     field is not one",
                ));
            };

            quote! {
                ::core::option::Option::Some(
                    <#inner as ::core::default::Default>::default()
                )
            }
        }
        // Annotated, so the overrides resolve against the type they
        // belong to rather than waiting on inference. Through an
        // `Option` they mean the inner value, which is the only thing
        // they could mean.
        Some(Start::Overrides(overrides)) => {
            let writes = overrides.iter().map(|over| {
                let member = &over.member;
                let value = &over.expr;
                quote!(value.#member = #value;)
            });

            from_default(ty, quote!(#(#writes)*))
        }
        // The attribute is the pattern it looks like, so the compiler
        // works out which position is which: `..` stands for however
        // many the type has left, which the macro cannot count.
        Some(Start::Positional(parts)) => {
            let target = option_inner(ty).unwrap_or(ty);

            let mut patterns = Vec::new();
            let mut writes = Vec::new();
            let mut rest = false;

            for (index, slot) in parts.iter().enumerate() {
                match slot {
                    Slot::Wild => patterns.push(quote!(_)),
                    Slot::Rest(token) if rest => {
                        return Err(syn::Error::new_spanned(
                            token,
                            "one `..` is all a pattern can have",
                        ));
                    }
                    Slot::Rest(_) => {
                        rest = true;
                        patterns.push(quote!(..));
                    }
                    Slot::Value(expr) => {
                        let at = format_ident!("__at{index}");
                        patterns.push(quote!(#at));
                        writes.push(quote!(*#at = #expr;));
                    }
                }
            }

            // A tuple's pattern is its shape. A tuple struct's names
            // the struct, and a pattern cannot reach it through an
            // alias, so the type is written out.
            let pattern = match target {
                Type::Tuple(_) => quote!((#(#patterns),*)),
                _ => {
                    let path = pattern_path(target)?;
                    quote!(#path(#(#patterns),*))
                }
            };

            from_default(
                ty,
                quote! {
                    let #pattern = &mut value;
                    #(#writes)*
                },
            )
        }
    })
}

/// A type as a pattern names it, which is with a turbofish where the
/// type as written has arguments.
fn pattern_path(ty: &Type) -> syn::Result<TokenStream2> {
    let Type::Path(path) = ty else {
        return Err(syn::Error::new(
            ty.span(),
            "a pattern of positions needs a tuple or a tuple struct",
        ));
    };

    let mut path = path.path.clone();

    for segment in &mut path.segments {
        if let PathArguments::AngleBracketed(args) =
            &mut segment.arguments
        {
            args.colon2_token = Some(Token![::](Span::call_site()));
        }
    }

    Ok(quote!(#path))
}

/// `writes` over the field's own default, and back as the field's
/// type. Through an `Option` the writes mean the value inside it,
/// which is the only thing they could mean.
fn from_default(ty: &Type, writes: TokenStream2) -> TokenStream2 {
    match option_inner(ty) {
        Some(inner) => quote! {{
            let mut value: #inner =
                ::core::default::Default::default();
            #writes
            ::core::option::Option::Some(value)
        }},
        None => quote! {{
            let mut value: #ty =
                ::core::default::Default::default();
            #writes
            value
        }},
    }
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

    let tokens = &attr.meta.require_list()?.tokens;

    // A leading `::` is the field's own type, left unwritten. The
    // tokens go on as they came, because a variant can be a path, a
    // call or a struct literal, and only one of those is a `Path`.
    let relative = attr.parse_args_with(|input: ParseStream| {
        let relative = input.peek(Token![::]);
        input.parse::<TokenStream2>()?;
        Ok(relative)
    })?;

    if relative {
        return Ok(Some(Start::Relative(tokens.clone())));
    }

    let slots = attr.parse_args_with(
        Punctuated::<Slot, Token![,]>::parse_terminated,
    );

    if let Ok(slots) = slots {
        // `..` on its own fills an `Option`, rather than being a
        // pattern that overrides nothing.
        if slots.len() == 1 && matches!(slots[0], Slot::Rest(_)) {
            return Ok(Some(Start::Present));
        }

        // A pattern, if any position of it is one. `_` and `..` are
        // the two that keep what the default had, and a position that
        // is neither is an expression, so nothing else can be meant.
        let holes = slots
            .iter()
            .any(|slot| matches!(slot, Slot::Wild | Slot::Rest(_)));

        if slots.len() > 1 && holes {
            return Ok(Some(Start::Positional(
                slots.into_iter().collect(),
            )));
        }
    }

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
