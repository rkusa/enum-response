extern crate enum_response;
extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

use std::str::FromStr;

use enum_response::StatusCode;
use proc_macro::TokenStream;
use quote::Tokens;
use syn::{Data, DataEnum, Fields, Ident, Lit, Meta, MetaList, MetaNameValue, NestedMeta};

#[proc_macro_derive(EnumResponse, attributes(response))]
pub fn enum_from(input: TokenStream) -> TokenStream {
    // parse the string representation
    let ast = syn::parse(input).unwrap();

    // derive the implementations
    let gen = derive(&ast);

    // return the generated impl
    gen.into()
}

enum ValueSource {
    String(String),
    TupleField(usize),
    StructField(String),
}

fn derive(ast: &syn::DeriveInput) -> quote::Tokens {
    let enum_name = &ast.ident;
    let variants = match ast.data {
        Data::Enum(DataEnum { ref variants, .. }) => variants,
        _ => panic!(
            "#[derive(EnumResponse)] can only be applied to enums. {} is not an enum.",
            enum_name
        ),
    };

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let mut status_patterns = Vec::new();
    let mut reason_patterns = Vec::new();

    'variants: for variant in variants {
        let variant_name = &variant.ident;

        let mut status = None;
        let mut reason = None;

        for attr in &variant.attrs {
            // TODO: unwrap
            match attr.interpret_meta().unwrap() {
                Meta::List(MetaList {
                    ref ident,
                    ref nested,
                    ..
                }) if ident == "response" =>
                {
                    // TODO: check paren_token?
                    for item in nested {
                        let (name, val) = match *item {
                            NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                                ref ident,
                                ref lit,
                                ..
                            })) => {
                                // TODO: check eq_token?
                                (ident, lit)
                            }
                            NestedMeta::Meta(Meta::Word(ref name)) => {
                                panic!("unknown response field attribute `{}`", name)
                            }
                            NestedMeta::Meta(Meta::List(MetaList { ref ident, .. })) => {
                                panic!("unknown response field attribute `{}`", ident)
                            }
                            NestedMeta::Literal(_) => {
                                panic!(
                                    "unexpected literal in response field attribute of `{}`",
                                    variant_name
                                );
                            }
                        };

                        match name.as_ref() {
                            "status" => {
                                status = Some(ValueSource::String(match val {
                                    &Lit::Int(ref status) => status_from_u16(status.value() as u16),
                                    &Lit::Str(ref name) => {
                                        let name = name.value();
                                        if let Ok(status) = u16::from_str(name.as_str()) {
                                            status_from_u16(status)
                                        } else {
                                            name
                                        }
                                    }
                                    _ => {
                                        panic!(
                                            "response status attribute value must be \
                                             of type int or string"
                                        );
                                    }
                                }));
                            }
                            "status_field" => {
                                match val {
                                    &Lit::Int(ref ix) => {
                                        status = Some(ValueSource::TupleField(ix.value() as usize));
                                    }
                                    &Lit::Str(ref s) => {
                                        let s = s.value();
                                        status = Some(match usize::from_str(&s) {
                                            Ok(ix) => ValueSource::TupleField(ix as usize),
                                            Err(_) => ValueSource::StructField(s),
                                        });
                                    }
                                    _ => {
                                        panic!(
                                            "response status attribute value must be \
                                             of type int or string"
                                        );
                                    }
                                };
                            }
                            "reason" => {
                                match val {
                                    &Lit::Str(ref s) => {
                                        reason = Some(ValueSource::String(s.value()));
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
                                    &Lit::Int(ref ix) => {
                                        reason = Some(ValueSource::TupleField(ix.value() as usize));
                                    }
                                    &Lit::Str(ref s) => {
                                        let s = s.value();
                                        reason = Some(match usize::from_str(&s) {
                                            Ok(ix) => ValueSource::TupleField(ix as usize),
                                            Err(_) => ValueSource::StructField(s),
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
                _ => continue,
            }
        }

        // TODO handling of status and reason are redundant

        match status {
            Some(ValueSource::String(ref status)) => {
                let pattern = variant_pattern(enum_name, variant_name, &variant.fields);
                let status = Ident::from(status.as_str());
                status_patterns.push(quote! {
                    #pattern => ::enum_response::StatusCode::#status,
                });
            }
            Some(ValueSource::TupleField(ix)) => {
                let fields = match variant.fields {
                    Fields::Unnamed(syn::FieldsUnnamed { ref unnamed, .. }) => unnamed,
                    _ => panic!("status index only works for tuple variants"),
                };

                if fields.iter().nth(ix).is_none() {
                    panic!(
                        "[error(status_field = {})]: No tuple field at {} found for {}",
                        ix, ix, variant_name
                    );
                }

                let fields = fields.iter().enumerate().map(|(i, _)| {
                    if i == ix {
                        quote! { status }
                    } else {
                        quote! { _ }
                    }
                });

                status_patterns.push(quote! {
                    #enum_name::#variant_name(#(#fields),*) => status,
                });
            }
            Some(ValueSource::StructField(field_name)) => {
                let fields = match variant.fields {
                    Fields::Named(syn::FieldsNamed { ref named, .. }) => named,
                    _ => panic!("status field only works for struct variants"),
                };

                let field = fields.iter().find(|f| {
                    if let Some(ref name) = f.ident {
                        name == &field_name
                    } else {
                        false
                    }
                });
                if let Some(field) = field {
                    let field_name = &field.ident;
                    status_patterns.push(quote! {
                        #enum_name::#variant_name { #field_name, .. } => #field_name,
                    });
                } else {
                    panic!(
                        "#[response(status_field = \"{}\")] struct field does not exist",
                        field_name
                    );
                }
            }
            None => {}
        }

        match reason {
            Some(ValueSource::String(reason)) => {
                let pattern = variant_pattern(enum_name, variant_name, &variant.fields);
                reason_patterns.push(quote! {
                    #pattern => Some(#reason),
                });
            }
            Some(ValueSource::TupleField(ix)) => {
                let fields = match variant.fields {
                    Fields::Unnamed(syn::FieldsUnnamed { ref unnamed, .. }) => unnamed,
                    _ => panic!("reason index only works for tuple variants"),
                };

                if fields.iter().nth(ix).is_none() {
                    panic!(
                        "[error(reason_field = {})]: No tuple field at {} found for {}",
                        ix, ix, variant_name
                    );
                }

                let fields = fields.iter().enumerate().map(|(i, _)| {
                    if i == ix {
                        quote! { ref reason }
                    } else {
                        quote! { _ }
                    }
                });

                reason_patterns.push(quote! {
                    #enum_name::#variant_name(#(#fields),*) => Some(reason),
                });
            }
            Some(ValueSource::StructField(field_name)) => {
                let fields = match variant.fields {
                    Fields::Named(syn::FieldsNamed { ref named, .. }) => named,
                    _ => panic!("reason field only works for struct variants"),
                };

                let field = fields.iter().find(|f| {
                    if let Some(ref name) = f.ident {
                        name == &field_name
                    } else {
                        false
                    }
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
            _ => ::enum_response::StatusCode::InternalServerError,
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
        impl #impl_generics ::enum_response::EnumResponse for #enum_name #ty_generics
            #where_clause
        {
            fn status(&self) -> ::enum_response::StatusCode {
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

fn variant_pattern(enum_name: &Ident, variant_name: &Ident, variant_data: &Fields) -> Tokens {
    match variant_data {
        &Fields::Unit => {
            quote! { #enum_name::#variant_name }
        }
        &Fields::Unnamed(_) => {
            quote! { #enum_name::#variant_name(..) }
        }
        &Fields::Named(_) => {
            quote! { #enum_name::#variant_name { .. } }
        }
    }
}
