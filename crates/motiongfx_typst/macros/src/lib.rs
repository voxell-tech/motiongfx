use proc_macro::TokenStream;
use quote::quote;
// use syn::{parse_macro_input, DeriveInput};

#[proc_macro]
pub fn typst(input: TokenStream) -> TokenStream {
    // let inputs = input.into_iter().collect::<Vec<_>>();

    for token in input {
        // println!("{:?}", token);
        match token {
            proc_macro::TokenTree::Literal(literal) => {
                println!("{}", literal.to_string());
            }
            _ => {}
        }
    }
    // let input = parse_macro_input!(input as DeriveInput);
    // let ident = input.ident;

    quote! {}.into()
}
