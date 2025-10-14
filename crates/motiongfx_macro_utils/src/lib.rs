use quote::quote;
use syn::DeriveInput;

pub fn get_one_field_of_attribute(input: &DeriveInput, attr_name: &str) -> syn::Ident {
    let syn::Data::Struct(struct_data) = &input.data else {
        panic!("Can only be implemented on a Struct.");
    };

    let field_filter: Vec<&syn::Field> = struct_data
        .fields
        .iter()
        .filter(|field| {
            field
                .attrs
                .iter()
                .filter(|attr| attr.path().is_ident(attr_name))
                .count()
                == 1
        })
        .collect();

    if field_filter.len() != 1 {
        panic!(
            "Expected exactly 1 field with #[{}] attribute. Given {}.",
            attr_name,
            field_filter.len()
        );
    }

    let shape_ident = field_filter[0].ident.as_ref().unwrap();
    shape_ident.clone()
}
