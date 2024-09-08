use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Fields, GenericArgument, Lit, Meta,
    MetaNameValue, NestedMeta, PathArguments,
};

#[proc_macro_derive(Model, attributes(model))]
pub fn model_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let mut has_relations = false;

    // Initialize the table name to a default or error message in case attribute is not found
    let mut table_name = None;

    let mut input_derives = Vec::new();

    // Iterate over the attributes to find `model` and then `table_name`
    for attr in input.attrs {
        if let Ok(Meta::List(meta)) = attr.parse_meta() {
            if meta.path.is_ident("model") {
                for nested_meta in meta.nested {
                    match nested_meta {
                        NestedMeta::Meta(Meta::Path(path)) if path.is_ident("has_relations") => {
                            has_relations = true;
                        }
                        NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                            path,
                            lit: Lit::Str(lit_str),
                            ..
                        })) => {
                            if path.is_ident("table_name") {
                                table_name = lit_str.into();
                            }
                        }
                        NestedMeta::Meta(Meta::List(meta_list)) => {
                            if meta_list.path.is_ident("input_derives") {
                                for nested in meta_list.nested {
                                    if let NestedMeta::Meta(Meta::Path(path)) = nested {
                                        input_derives.push(path);
                                    }
                                }
                            }
                        }
                        _ => panic!("Invalid attribute"),
                    }
                }
            }
        }
    }

    let fields = match input.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,
            _ => panic!("Model only supports named fields"),
        },
        _ => panic!("Model can only be derived for structs"),
    };

    let input_visibility = &input.vis;
    let input_ident = syn::Ident::new(&format!("{}Input", &ident), ident.span());
    let input_generics = &input.generics;

    let input_fields = fields
        .clone()
        .into_iter()
        .filter(|f| {
            let (id, _, _, _, _, _, _) = parse_attributes(&f.attrs);

            !id
        })
        .map(|mut field| {
            field.attrs.retain(|attr| match attr.parse_meta() {
                Ok(Meta::List(meta_list)) => !meta_list.path.is_ident("model"),
                _ => true,
            });

            field
        })
        .collect::<Vec<_>>();

    let input_derives = if input_derives.is_empty() {
        quote! {}
    } else {
        quote! {
            #[derive(#(#input_derives),*)]
        }
    };

    let input_type_definition = quote! {
        #input_derives
        #input_visibility struct #input_ident #input_generics {
            #(#input_fields),*
        }
    };

    let table_name = table_name.expect("Specify #[model(table_name = \"...\")] attribute");

    // find the id field
    let mut field_attributes = fields.iter().map(|f| (f, parse_attributes(&f.attrs)));

    let mut id_field: Option<Ident> = None;
    let mut primary_key_set = false;
    while let Some((f, (id, _, _, _, primary_key, _, _))) = field_attributes.next() {
        if id && id_field.is_some() {
            panic!("only one field may be declared as the id field")
        }

        if id {
            id_field = f.ident.clone();
        }

        if primary_key {
            primary_key_set = true;
        }
    }

    if !primary_key_set {
        panic!("at least one field must be set as the primary key");
    }

    let id_field =
        id_field.expect("at least one field of type Uuid must be declared as the id field");

    let id_field_string = id_field.to_string();

    let (field_definitions, field_value_getters): (Vec<_>, Vec<_>) = fields
        .iter()
        .filter_map(|f| {
            let name_ident = f.ident.as_ref().unwrap();
            let name = name_ident.to_string();
            let (id, json, skip, immutable, primary_key, unique, enum_) = parse_attributes(&f.attrs);

            if id {
                if skip {
                    panic!("Field declared as id can't be skipped");
                }

                if json {
                    panic!("Field declared as id can't be json");
                }
            }

            if skip {
                return None;
            }

            let (field_type, nesting) = map_field_type(&f.ty, id, json, enum_, 0);

            let nullable = nesting > 0;

            let field_definition = quote! {
                model::FieldDefinition {
                    name: String::from(#name),
                    type_: #field_type,
                    immutable: #immutable,
                    primary_key: #primary_key,
                    unique: #unique,
                    nullable: #nullable,
                }
            };

            let value_as_option = if nullable {
                flatten_option(nesting)
            } else {
                quote! {
                    let value = Some(value);
                }
            };

            let return_result_field_value = if json {
                quote! {
                    if let Some(value) = value {
                        let value = model::serde_json::to_value(value).map_err(|_| model::Error::bad_request("unable to serialize field into json"))?;
                        return Ok(Some(value).into());
                    }

                    Ok(model::FieldValue::Json(None))
                }
            } else {
                quote! {
                    Ok(value.into())
                }
            };

            let field_value_getter = quote! {
                #name => {
                    let value = self.#name_ident.clone();

                    #value_as_option
                    #return_result_field_value
                },
            };

            Some((field_definition, field_value_getter))
        })
        .unzip();

    let input_struct_assignments: Vec<_> = input_fields
        .iter()
        .map(|field| {
            let field_ident = field.ident.as_ref().expect("fields must be named");

            quote! {
                #field_ident: input.#field_ident
            }
        })
        .collect();

    let input_assignments: Vec<_> = input_fields
        .iter()
        .filter(|field| {
            let (_, _, _, immutable, _, _, _) = parse_attributes(&field.attrs);
            !immutable
        })
        .map(|field| {
            let field_ident = field.ident.as_ref().expect("fields must be named");

            quote! {
                self.#field_ident = input.#field_ident;
            }
        })
        .collect();

    let input_impl = quote! {
        impl #impl_generics model::Input for #ident #type_generics #where_clause {
            type InputType = #input_ident;

            fn from_input(input: Self::InputType) -> Self {
                Self {
                    #id_field: uuid::Uuid::new_v4(),
                    #(#input_struct_assignments),*
                }
            }

            fn merge_input(mut self, input: Self::InputType) -> Self {
                #(#input_assignments)*

                self
            }
        }
    };

    let model_impl = quote! {
        impl #impl_generics model::Model for #ident #type_generics #where_clause {
            fn table_name() -> String {
                #table_name.into()
            }

            fn id_field_name() -> String {
                #id_field_string.into()
            }

            fn field_definitions() -> Vec<model::FieldDefinition> {
                vec![#(#field_definitions),*]
            }

            fn id_field_value(&self) -> Uuid {
                self.#id_field.clone()
            }

            fn field_value(&self, field: &str) -> Result<model::FieldValue, model::Error> {
                match field {
                    #(#field_value_getters)*
                    _ => Err(model::Error::bad_request("invalid field name"))
                }
            }
        }
    };

    let related_impl = if has_relations {
        quote! {}
    } else {
        quote! {
            impl #impl_generics model::Related for #ident #type_generics #where_clause {}
        }
    };

    let out = quote! {
        #input_type_definition

        #input_impl

        #model_impl

        #related_impl
    };

    out.into()
}

fn flatten_option(mut nesting: usize) -> proc_macro2::TokenStream {
    let mut accumulated = quote!();

    while nesting > 1 {
        accumulated = quote! {
            #accumulated
            let value = value.flatten();
        };

        nesting -= 1;
    }

    accumulated
}

fn map_field_type(
    ty: &syn::Type,
    id: bool,
    json: bool,
    enum_: bool,
    level: usize,
) -> (proc_macro2::TokenStream, usize) {
    use syn::{Type, TypePath};

    match ty {
        Type::Path(TypePath { path, .. }) => {
            let last_segment = path.segments.last().unwrap();
            let ident = &last_segment.ident;
            let ident_str = ident.to_string();

            if id {
                if ident_str != "Uuid" {
                    panic!("Field declared as id must be of type Uuid")
                }

                return (quote!(model::FieldType::Uuid), level);
            }

            if ident_str == "Option" {
                if let PathArguments::AngleBracketed(angle_bracketed_param) =
                    &last_segment.arguments
                {
                    if let Some(GenericArgument::Type(inner_type)) =
                        angle_bracketed_param.args.first()
                    {
                        // Handle the inner type, which might be a complex path
                        return map_field_type(inner_type, id, json, enum_, level + 1);
                    }
                }

                panic!("unsupported field type")
            } else {
                if json {
                    (quote!(model::FieldType::Json), level)
                } else {
                    (map_field_inner_type(ident, enum_), level)
                }
            }
        }
        _ => panic!("unsupported field type!"), // Default or error
    }
}

fn map_field_inner_type(inner_type: &Ident, enum_: bool) -> proc_macro2::TokenStream {
    // handle the enum case if the enum attribute is set
    if enum_ {
        return quote! {
            model::FieldType::Enum(#inner_type::variants())
        };
    }

    match inner_type.to_string().as_str() {
        "Uuid" => quote! { model::FieldType::Uuid },
        "bool" => quote! { model::FieldType::Bool },
        "i64" => quote! { model::FieldType::Int },
        "i32" => quote! { model::FieldType::Int32 },
        "f64" => quote! { model::FieldType::Float },
        "Decimal" => quote! { model::FieldType::Decimal },
        "String" => quote! { model::FieldType::String },
        "NaiveDate" => quote! { model::FieldType::Date },
        "DateTime" => quote! { model::FieldType::DateTime },
        _ => panic!("unsupported field type: {}. If this is an Enum, please mark the field with the enum attribute: #[model(enum)]", inner_type), // Default or error
    }
}

fn parse_attributes(attrs: &[Attribute]) -> (bool, bool, bool, bool, bool, bool, bool) {
    let mut id = false;
    let mut json = false;
    let mut skip = false;
    let mut immutable = false;
    let mut primary_key = false;
    let mut unique = false;
    let mut enum_ = false;

    for attr in attrs {
        if let Ok(Meta::List(meta)) = attr.parse_meta() {
            if meta.path.is_ident("model") {
                for nested in meta.nested {
                    match nested {
                        syn::NestedMeta::Meta(Meta::Path(path)) if path.is_ident("id") => {
                            id = true;
                            unique = true;
                            immutable = true;
                        }
                        syn::NestedMeta::Meta(Meta::Path(path)) if path.is_ident("json") => {
                            json = true;
                        }
                        syn::NestedMeta::Meta(Meta::Path(path)) if path.is_ident("skip") => {
                            skip = true;
                        }
                        syn::NestedMeta::Meta(Meta::Path(path)) if path.is_ident("immutable") => {
                            immutable = true;
                        }
                        syn::NestedMeta::Meta(Meta::Path(path)) if path.is_ident("primary_key") => {
                            primary_key = true;
                        }
                        syn::NestedMeta::Meta(Meta::Path(path)) if path.is_ident("unique") => {
                            unique = true;
                        }
                        syn::NestedMeta::Meta(Meta::Path(path)) if path.is_ident("enum") => {
                            enum_ = true;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    (id, json, skip, immutable, primary_key, unique, enum_)
}
