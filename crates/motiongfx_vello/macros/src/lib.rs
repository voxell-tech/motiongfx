use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(VelloBuilder)]
pub fn vello_builder_derive_macro(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;

    quote! {
        impl VelloBuilder for #ident {
            #[inline]
            fn is_built(&self) -> bool {
                self.built
            }

            #[inline]
            fn set_built(&mut self, built: bool) {
                self.built = built;
            }
        }
    }
    .into()
}

#[proc_macro_derive(VelloVector, attributes(shape))]
pub fn vello_vector_derive_macro(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let syn::Data::Struct(struct_data) = &mut input.data else {
        panic!("Can only be implemented on a Struct.");
    };

    let field_filter: Vec<&syn::Field> = struct_data
        .fields
        .iter()
        .filter(|field| {
            field
                .attrs
                .iter()
                .filter(|attr| attr.path().is_ident("shape"))
                .count()
                == 1
        })
        .collect();

    if field_filter.len() != 1 {
        panic!(
            "Expected only 1 field with #[shape] attribute. Given {}.",
            field_filter.len()
        );
    } else {
        let shape_ident = field_filter[0].ident.as_ref().unwrap();

        quote!(
            impl #impl_generics VelloVector for #ident #type_generics #where_clause {
                #[inline]
                fn shape(&self) -> &impl bevy_vello_renderer::vello::kurbo::Shape {
                    &self.#shape_ident
                }
            }
        )
        .into()
    }
}
