use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{parse_macro_input, DeriveInput, Data, Fields, Ident, Type, Token, braced};
use syn::parse::{Parse, ParseStream};

struct MotionPathsDef {
    structs: Vec<MotionStruct>,
}

struct MotionStruct {
    name: Ident,
    fields: Vec<MotionField>,
}

struct MotionField {
    name: Ident,
    ty: Type,
}

impl Parse for MotionPathsDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut structs = Vec::new();
        while !input.is_empty() {
            let motion_struct = input.parse::<MotionStruct>()?;
            structs.push(motion_struct);
            if input.peek(Token![,]) {
                let _ = input.parse::<Token![,]>()?;
            }
        }
        Ok(MotionPathsDef { structs })
    }
}

impl Parse for MotionStruct {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        let content;
        braced!(content in input);
        let mut fields = Vec::new();
        while !content.is_empty() {
            fields.push(content.parse::<MotionField>()?);
            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>()?;
            }
        }
        Ok(MotionStruct { name, fields })
    }
}

impl Parse for MotionField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty = input.parse()?;
        Ok(MotionField { name, ty })
    }
}

fn to_snake_case(name: &str) -> String {
    let mut snake = String::new();
    for (i, ch) in name.char_indices() {
        if i > 0 && ch.is_uppercase() {
            snake.push('_');
        }
        snake.push(ch.to_ascii_lowercase());
    }
    snake
}

fn get_type_ident(ty: &Type) -> Option<Ident> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident_str = segment.ident.to_string();
            match ident_str.as_str() {
                "f32" | "f64" | "u8" | "u16" | "u32" | "u64" | "usize" |
                "i8" | "i16" | "i32" | "i64" | "isize" | "bool" | "Option" | "Vec" | "Option" => return None,
                _ => {}
            }
            if ident_str.chars().next().map_or(false, |c| c.is_uppercase()) {
                return Some(segment.ident.clone());
            }
        }
    }
    None
}

#[proc_macro]
pub fn motion_paths(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input as MotionPathsDef);
    let mut expanded = proc_macro2::TokenStream::new();
    
    for strct in &def.structs {
        let root_type = &strct.name;
        let mod_name = format_ident!("{}", to_snake_case(&root_type.to_string()));
        let macro_name = format_ident!("__motion_paths_{}", root_type);
        
        let mut fields_tokens = proc_macro2::TokenStream::new();
        
        for field in &strct.fields {
            let name = &field.name;
            let ty = &field.ty;
            let root_alias = format_ident!("__Root_For_{}", name);
            let ty_alias = format_ident!("__Ty_For_{}", name);
            
            let nested_expansion = if let Some(ty_ident) = get_type_ident(ty) {
                let nested_macro_name = format_ident!("__motion_paths_{}", ty_ident);
                quote! {
                    #nested_macro_name!(super::#root_alias, $($path)* . #name);
                }
            } else {
                quote! {}
            };

            fields_tokens.extend(quote! {
                #[allow(non_camel_case_types)]
                pub type #root_alias = $root;
                #[allow(non_camel_case_types)]
                pub type #ty_alias = #ty;
                
                pub mod #name {
                    #[allow(non_upper_case_globals)]
                    pub const PATH: ::motiongfx::field_path::path::Path<super::#root_alias, super::#ty_alias> = ::motiongfx::field_path::path!(super::#root_alias $($path)* . #name);
                    
                    #nested_expansion
                }
                #[allow(non_upper_case_globals)]
                pub const #name: ::motiongfx::field_path::path::Path<$root, #ty> = #name::PATH;
            });
        }
        
        expanded.extend(quote! {
            #[macro_export]
            macro_rules! #macro_name {
                ($root:ty, $($path:tt)*) => {
                    #fields_tokens
                }
            }
            
            pub mod #mod_name {
                #[allow(unused_imports)]
                use super::*;
                #macro_name!(super::#root_type, );
            }
        });
    }
    
    TokenStream::from(expanded)
}

#[proc_macro_derive(MotionPaths)]
pub fn derive_motion_paths(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let mod_name = format_ident!("{}", to_snake_case(&name.to_string()));
    let macro_name = format_ident!("__motion_paths_{}", name);

    let mut fields_tokens = proc_macro2::TokenStream::new();

    if let Data::Struct(data) = &input.data {
        if let Fields::Named(fields) = &data.fields {
            for field in &fields.named {
                let field_name = field.ident.as_ref().unwrap();
                let field_ty = &field.ty;
                let root_alias = format_ident!("__Root_For_{}", field_name);
                let ty_alias = format_ident!("__Ty_For_{}", field_name);
                
                let nested_expansion = if let Some(ty_ident) = get_type_ident(field_ty) {
                    let nested_macro_name = format_ident!("__motion_paths_{}", ty_ident);
                    quote! {
                        #nested_macro_name!(super::#root_alias, $($path)* . #field_name);
                    }
                } else {
                    quote! {}
                };

                fields_tokens.extend(quote! {
                    #[allow(non_camel_case_types)]
                    pub type #root_alias = $root;
                    #[allow(non_camel_case_types)]
                    pub type #ty_alias = #field_ty;
                    
                    pub mod #field_name {
                        #[allow(non_upper_case_globals)]
                        pub const PATH: ::motiongfx::field_path::path::Path<super::#root_alias, super::#ty_alias> = ::motiongfx::field_path::path!(super::#root_alias $($path)* . #field_name);
                        #nested_expansion
                    }
                    #[allow(non_upper_case_globals)]
                    pub const #field_name: ::motiongfx::field_path::path::Path<$root, #field_ty> = #field_name::PATH;
                });
            }
        }
    }

    let expanded = quote! {
        #[macro_export]
        macro_rules! #macro_name {
            ($root:ty, $($path:tt)*) => {
                #fields_tokens
            }
        }
        
        pub mod #mod_name {
            #![allow(non_upper_case_globals)]
            #[allow(unused_imports)]
            use super::*;
            #macro_name!(super::#name, );
        }
    };

    TokenStream::from(expanded)
}
