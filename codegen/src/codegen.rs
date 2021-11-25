use super::types;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use suds_wsdl::types::{self as wsdl, NamespacedName, Namespaces};

pub trait Codegen {
    fn codegen(&self, namespaces: &Namespaces) -> TokenStream;
}

fn codegen_all(all: &[impl Codegen], namespaces: &Namespaces) -> Vec<TokenStream> {
    all.iter().map(|item| item.codegen(namespaces)).collect()
}

impl Codegen for types::Definition {
    fn codegen(&self, namespaces: &Namespaces) -> TokenStream {
        let types = codegen_all(&self.types, namespaces);
        let messages = codegen_all(&self.messages, namespaces);
        let services = codegen_all(&self.services, namespaces);

        quote! {
            pub mod types {
                #(#types)*
            }

            pub mod messages {
                #(#messages)*
            }

            pub mod services {
                #(#services)*
            }
        }
    }
}

impl Codegen for wsdl::Type {
    fn codegen(&self, namespaces: &Namespaces) -> TokenStream {
        let name = format_ident!("{}", &self.name.name);
        let fields = match &self.kind {
            wsdl::TypeKind::Struct(fields) => codegen_all(fields, namespaces),
        };

        quote! {
            #[derive(Debug, Clone)]
            pub struct #name {
                #(#fields)*
            }
        }
    }
}

impl Codegen for wsdl::Field {
    fn codegen(&self, namespaces: &Namespaces) -> TokenStream {
        let name = format_ident!("{}", &self.name.name);

        let ty = if let Some(ident) = match &self.ty.name as &str {
            "int" => Some(format_ident!("isize")),
            _ => None,
        } {
            quote! {#ident}
        } else {
            let ident = format_ident!("{}", &self.ty.name);
            quote! { super::types::#ident }
        };

        quote! {
            pub #name: #ty,
        }
    }
}

impl Codegen for wsdl::Message {
    fn codegen(&self, namespaces: &Namespaces) -> TokenStream {
        let name = format_ident!("{}", &self.name.name);
        let fields = codegen_all(&self.parts, namespaces);

        quote! {
            #[derive(Debug, Clone)]
            pub struct #name {
                #(#fields)*
            }
        }
    }
}

impl Codegen for types::Service {
    fn codegen(&self, namespaces: &Namespaces) -> TokenStream {
        let name = format_ident!("{}", &self.name.name);
        let ports = codegen_all(&self.ports, namespaces);

        quote! {
            pub mod #name {
                #(#ports)*
            }
        }
    }
}

impl Codegen for types::Port {
    fn codegen(&self, namespaces: &Namespaces) -> TokenStream {
        let name = format_ident!("{}", &self.name.name);
        let location = &self.location;
        let operations = codegen_all(&self.operations, namespaces);

        quote! {
            pub struct #name {
                location: &'static str
            }

            impl #name {
                pub fn new() -> Self {
                    Self {
                        location: #location
                    }
                }

                #(#operations)*
            }
        }
    }
}

impl Codegen for wsdl::Operation {
    fn codegen(&self, namespaces: &Namespaces) -> TokenStream {
        let name = format_ident!("{}", &self.name.name);

        let input = if let Some(input) = &self.input {
            let ident = format_ident!("{}", &input.name);
            quote! {
                , input: super::super::messages::#ident
            }
        } else {
            quote! {}
        };

        let output = if let Some(output) = &self.output {
            let ident = format_ident!("{}", &output.name);
            quote! {
                -> super::super::messages::#ident
            }
        } else {
            quote! {}
        };

        quote! {
            pub fn #name(&self #input) #output {
                unimplemented!()
            }
        }
    }
}
