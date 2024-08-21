use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Enum, attributes(model))]
pub fn enum_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Extract the variant names
    let variants = match &input.data {
        syn::Data::Enum(data_enum) => data_enum
            .variants
            .iter()
            .map(|variant| &variant.ident)
            .collect::<Vec<_>>(),
        _ => {
            panic!("Enum can only be used with enums")
        }
    };

    let variant_strings = variants.iter().map(|v| v.to_string()).collect::<Vec<_>>();

    let variant_to_string = variants
        .iter()
        .zip(variant_strings.iter())
        .collect::<Vec<_>>();

    let from_string_arms = variant_to_string
        .iter()
        .map(|(variant, variant_string)| {
            quote! {
                #variant_string => Ok(#name::#variant),
            }
        })
        .collect::<Vec<_>>();

    let to_string_arms = variant_to_string
        .iter()
        .map(|(variant, variant_string)| {
            quote! {
                #name::#variant => #variant_string.to_string(),
            }
        })
        .collect::<Vec<_>>();

    // Generate the implementation of the Enum trait
    let expanded = quote! {
        impl model::Enum for #name {
            fn variants() -> Vec<String> {
                vec![
                    #(#variant_strings.to_string(),)*
                ]
            }

            fn try_from_string(value: String) -> Result<Self, model::Error> {
                match value.as_str() {
                    #(#from_string_arms)*
                    _ => Err(model::Error::bad_request("invalid enum variant"))
                }
            }

            fn to_string(self) -> String {
                match self {
                    #(#to_string_arms)*
                }
            }
        }

        impl model::sqlx::Type<model::sqlx::Postgres> for #name {
            fn type_info() -> <model::sqlx::Postgres as model::sqlx::Database>::TypeInfo {
                model::sqlx::postgres::PgTypeInfo::with_name("text")
            }
        }

        impl<'r> model::sqlx::Decode<'r, model::sqlx::Postgres> for #name {
            fn decode(
                // value: <model::sqlx::Postgres as model::sqlx::database::HasValueRef<'r>>::ValueRef,
                value: model::sqlx::postgres::PgValueRef<'r>,
            ) -> Result<Self, model::sqlx::error::BoxDynError> {
                let variant_string = <String as model::sqlx::Decode<model::sqlx::Postgres>>::decode(value)?;

                Ok(#name::try_from_string(variant_string)?)
            }
        }
    };

    // Return the generated implementation as a TokenStream
    TokenStream::from(expanded)
}
