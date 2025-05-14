use std::str::FromStr;

use proc_macro::TokenStream;
use proc_macro2::{
    Punct, Spacing, Span, TokenStream as TokenStream2,
};
use quote::{quote, TokenStreamExt};
use syn::parse::{Parse, ParseStream};
use syn::token::Comma;
use syn::{parse_macro_input, Ident, LitInt, Result};

struct CombinationTuple {
    macro_ident: Ident,
    count: usize,
}

impl Parse for CombinationTuple {
    fn parse(input: ParseStream) -> Result<Self> {
        let macro_ident = input.parse::<Ident>()?;
        input.parse::<Comma>()?;
        let count = input.parse::<LitInt>()?.base10_parse()?;

        Ok(CombinationTuple { macro_ident, count })
    }
}

fn generate_tuple_combinations(
    count: usize,
) -> (Vec<Ident>, TokenStream2) {
    assert!(count > 0, "Number of generics must be greater than 0.");

    let mut tuple_idents = Vec::with_capacity(count);
    for i in 0..count {
        let ident = Ident::new(&format!("T{i}"), Span::call_site());
        tuple_idents.push(ident);
    }

    let mut generics = TokenStream2::new();
    for ident in tuple_idents.iter().take(tuple_idents.len() - 1) {
        generics.append(ident.clone());
        generics.append(Punct::new(',', Spacing::Alone));
    }

    // SAFETY: `count` is guaranteed to be more than 0.
    generics.append(tuple_idents.last().unwrap().clone());
    (tuple_idents, generics)
}

#[proc_macro]
pub fn tuple_combinations(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as CombinationTuple);
    let macro_ident = input.macro_ident;
    assert!(
        input.count > 1,
        "Number of generics must be greater than 1."
    );

    let mut tokens = TokenStream2::new();

    for c in 2..=input.count {
        let (tuple_idents, generics) = generate_tuple_combinations(c);

        for (i, tuple_ident) in tuple_idents.iter().enumerate() {
            let number_token =
                TokenStream2::from_str(&format!("{i}")).unwrap();
            let q = quote! {
                #macro_ident!([#generics], #tuple_ident, #number_token);
            };

            tokens.extend(q);
        }
    }

    tokens.into()
}
