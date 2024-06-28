use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Ident, Token};

struct IdentsInput {
    idents: Vec<Ident>,
}

impl Parse for IdentsInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let idents = input.parse_terminated::<Ident, Token![,]>(Ident::parse)?;
        Ok(IdentsInput {
            idents: idents.into_iter().collect(),
        })
    }
}

#[proc_macro]
pub fn model_wrapper(input: TokenStream) -> TokenStream {
    let IdentsInput { idents } = parse_macro_input!(input as IdentsInput);

    let enum_variants = idents.iter().map(|ident| {
        quote! {
            #ident(#ident)
        }
    });

    let table_name_matches = idents.iter().map(|ident| {
        quote! {
            if table_name == &#ident::table_name() {
                return Ok(ModelWrapper::#ident(#ident::from_pgoutput(col_names, row)?));
            }
        }
    });

    let generated = quote! {
        #[derive(Debug)]
        pub enum ModelWrapper {
            #(#enum_variants),*
        }

        impl ModelWrapper {
            pub fn from_pgoutput(table_name: &str, col_names: &Vec<String>, row: Vec<Option<String>>) -> Result<Self, model::Error> {
                use model::FromPgoutput;

                #(#table_name_matches)*
                Err(model::Error::bad_request(&format!("unknown tablename {}", table_name)))
            }
        }
    };

    TokenStream::from(generated)
}
