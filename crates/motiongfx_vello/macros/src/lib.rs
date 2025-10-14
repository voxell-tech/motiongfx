use motiongfx_macro_utils::get_one_field_of_attribute;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(VelloBuilder)]
pub fn vello_builder_derive_macro(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let struct_name = &ast.ident;

    quote! {
        impl VelloBuilder for #struct_name {
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
    let ast = parse_macro_input!(input as DeriveInput);

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = ast.generics.split_for_impl();

    let shape_ident = get_one_field_of_attribute(&ast, "shape");

    quote!(
        impl #impl_generics VelloVector for #struct_name #type_generics #where_clause {
            #[inline]
            fn shape(&self) -> &impl bevy_vello_renderer::vello::kurbo::Shape {
                &self.#shape_ident
            }
        }
    )
    .into()
}
