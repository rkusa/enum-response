// #[recursion_limit="128"]

extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;
extern crate api_error;

use std::str::FromStr;

use api_error::StatusCode;
use proc_macro::TokenStream;
use quote::Tokens;
use syn::{Body, VariantData, MetaItem, NestedMetaItem, Lit, Ident};

#[proc_macro_derive(ErrorStatus, attributes(response))]
pub fn enum_from(input: TokenStream) -> TokenStream {
    // construct a string representation of the type definition
    let s = input.to_string();

    // parse the string representation
    let ast = syn::parse_derive_input(&s).unwrap();

    // derive the implementations
    let gen = derive(&ast);

    // return the generated impl
    gen.parse().unwrap()
}

enum ReasonSource<'a> {
    String(&'a str),
    TupleField(usize),
    StructField(&'a str),
}

fn derive(ast: &syn::DeriveInput) -> quote::Tokens {
    let enum_name = &ast.ident;
    let variants = match ast.body {
        Body::Enum(ref variants) => variants,
        _ => {
            panic!(
                "#[derive(EnumFrom)] can only be applied to enums. {} is not an enum.",
                enum_name
            )
        }
    };

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let mut status_patterns = Vec::new();
    let mut reason_patterns = Vec::new();

    'variants: for variant in variants {
        let variant_name = &variant.ident;

        let mut status = None;
        let mut reason = None;

        for attr in variant.attrs.iter() {
            let items = match attr.value {
                MetaItem::List(ref k, ref items) if k == "response" => items,
                _ => continue,
            };
            for item in items {
                let (name, val) = match item {
                    &NestedMetaItem::MetaItem(MetaItem::NameValue(ref name, ref val)) => {
                        (name, val)
                    }
                    &NestedMetaItem::MetaItem(MetaItem::Word(ref name)) => {
                        panic!("unknown response field attribute `{}`", name)
                    }
                    &NestedMetaItem::MetaItem(MetaItem::List(ref name, _)) => {
                        panic!("unknown response field attribute `{}`", name)
                    }
                    &NestedMetaItem::Literal(_) => {
                        panic!(
                            "unexpected literal in response field attribute of `{}`",
                            variant_name
                        );
                    }
                };

                match name.as_ref() {
                    "status" => {
                        status = Some(Ident::new(match val {
                            &Lit::Int(status, _) => status_from_u16(status as u16),
                            &Lit::Str(ref name, _) => {
                                if let Ok(status) = u16::from_str(name) {
                                    status_from_u16(status)
                                } else {
                                    name.clone()
                                }
                            }
                            _ => {
                                panic!(
                                    "response reason attribute value must be \
                                    of type int or string"
                                );
                            }
                        }));
                    }
                    "reason" => {
                        match val {
                            &Lit::Str(ref s, _) => {
                                reason = Some(ReasonSource::String(s));
                            }
                            _ => {
                                panic!(
                                    "response reason attribute value must be \
                                    of type string"
                                );
                            }
                        };
                    }
                    "reason_field" => {
                        match val {
                            &Lit::Int(ix, _) => {
                                reason = Some(ReasonSource::TupleField(ix as usize));
                            }
                            &Lit::Str(ref s, _) => {
                                reason = Some(match usize::from_str(s) {
                                    Ok(ix) => ReasonSource::TupleField(ix as usize),
                                    Err(_) => ReasonSource::StructField(s),
                                });
                            }
                            _ => {
                                panic!(
                                    "response reason attribute value must be \
                                    of type int or string"
                                );
                            }
                        };
                    }
                    _ => panic!("unknown response field attribute `{}`", name),
                }
            }
        }

        if let Some(status) = status {
            let pattern = variant_pattern(enum_name, variant_name, &variant.data);
            status_patterns.push(quote! {
                #pattern => ::api_error::StatusCode::#status,
            });
        }

        match reason {
            Some(ReasonSource::String(reason)) => {
                let pattern = variant_pattern(enum_name, variant_name, &variant.data);
                reason_patterns.push(quote! {
                    #pattern => Some(#reason),
                });
            }
            Some(ReasonSource::TupleField(ix)) => {
                let fields = match variant.data {
                    VariantData::Tuple(ref fields) => fields,
                    _ => panic!("reason index only works for tuple variants"),
                };

                if fields.get(ix).is_none() {
                    panic!(
                        "[error(reason_field = {})]: No tuple field at {} found for {}",
                        ix,
                        ix,
                        variant_name
                    );
                }

                let fields = fields.iter().enumerate().map(|(i, _)| {
                    Ident::new(if i == ix { "ref reason" } else { "_" })
                });

                reason_patterns.push(quote! {
                    #enum_name::#variant_name(#(#fields),*) => Some(reason),
                });
            }
            Some(ReasonSource::StructField(field_name)) => {
                let fields = match variant.data {
                    VariantData::Struct(ref fields) => fields,
                    _ => panic!("reason field only works for struct variants"),
                };

                let field = fields.iter().find(|f| if let Some(ref name) = f.ident {
                    name == field_name
                } else {
                    false
                });
                if let Some(field) = field {
                    let field_name = &field.ident;
                    reason_patterns.push(quote! {
                        #enum_name::#variant_name { #field_name, .. } => Some(#field_name),
                    });
                } else {
                    panic!(
                        "#[response(reason_field = \"{}\")] struct field does not exist",
                        field_name
                    );
                }
            }
            None => {}
        }
    }

    if status_patterns.len() < variants.len() {
        status_patterns.push(quote! {
            _ => ::api_error::StatusCode::InternalServerError,
        });
    }

    if reason_patterns.len() < variants.len() {
        reason_patterns.push(quote! {
            _ => self.status().canonical_reason(),
        });
    }

    let mut status_tokens = Tokens::new();
    status_tokens.append_all(status_patterns);

    let mut reason_tokens = Tokens::new();
    reason_tokens.append_all(reason_patterns);

    quote! {
        impl #impl_generics ::api_error::ErrorStatus for #enum_name #ty_generics
            #where_clause
        {
            fn status(&self) -> ::api_error::StatusCode {
                match *self {
                    #status_tokens
                }
            }

            fn reason(&self) -> Option<&str> {
                match *self {
                    #reason_tokens
                }
            }
        }
    }
}

fn status_from_u16(status: u16) -> String {
    format!("{:?}", StatusCode::try_from(status).unwrap())
}

fn variant_pattern(enum_name: &Ident, variant_name: &Ident, variant_data: &VariantData) -> Tokens {
    match variant_data {
        &VariantData::Unit => {
            quote! { #enum_name::#variant_name }
        }
        &VariantData::Tuple(_) => {
            quote! { #enum_name::#variant_name(..) }
        }
        &VariantData::Struct(_) => {
            quote! { #enum_name::#variant_name { .. } }
        }
    }
}
