use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use suds_wsdl::types::{Definition, Field, Message, Namespaces, Type, TypeKind};

pub trait Codegen {
    fn codegen(&self, namespaces: &Namespaces) -> TokenStream;
}

fn codegen_all(all: &[impl Codegen], namespaces: &Namespaces) -> Vec<TokenStream> {
    all.iter().map(|item| item.codegen(namespaces)).collect()
}

impl Codegen for Definition {
    fn codegen(&self, namespaces: &Namespaces) -> TokenStream {
        let types = codegen_all(&self.types, namespaces);
        let messages = codegen_all(&self.messages, namespaces);

        quote! {
            pub mod types {
                #(#types)*
            }

            pub mod messages {
                #(#messages)*
            }
        }
    }
}

impl Codegen for Type {
    fn codegen(&self, namespaces: &Namespaces) -> TokenStream {
        let name = format_ident!("{}", &self.name.name);
        let fields = match &self.kind {
            TypeKind::Struct(fields) => codegen_all(fields, namespaces),
        };

        quote! {
            #[derive(Debug, Clone)]
            pub struct #name {
                #(#fields)*
            }
        }
    }
}

impl Codegen for Field {
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

impl Codegen for Message {
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
