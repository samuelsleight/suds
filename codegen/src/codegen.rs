use super::types;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use suds_wsdl::types::{self as wsdl, Namespaces};

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
        let (fields, to_xml_fields, from_xml_fields) = match &self.kind {
            wsdl::TypeKind::Struct(fields) => (
                codegen_all(fields, namespaces),
                codegen_to_xml_fields(fields, namespaces),
                codegen_from_xml_fields(fields, namespaces),
            ),
        };

        let to_xml_name = format!("ns{}:{}", self.name.index(), &self.name.name);
        let from_xml_name = &self.name.name;

        let namespaces = namespaces
            .namespaces()
            .iter()
            .enumerate()
            .map(|(idx, url)| {
                let ns = format!("xmlns:ns{}", idx);
                quote! {.with_attributes([(#ns, #url)])}
            })
            .collect::<Vec<_>>();

        quote! {
            #[derive(Debug, Clone)]
            pub struct #name {
                #(#fields)*
            }

            impl suds_util::xml::ToXml for #name {
                fn to_xml<W: std::io::Write>(&self, writer: &mut suds_util::xml::Writer<W>, mut top_level: bool) {
                    let start = suds_util::xml::events::BytesStart::owned_name(#to_xml_name);

                    let start = if top_level {
                        start #(#namespaces)*
                    } else {
                        start
                    };

                    top_level = false;

                    writer.write_event(suds_util::xml::events::Event::Start(start.to_borrowed())).unwrap();
                    #(#to_xml_fields)*
                    writer.write_event(suds_util::xml::events::Event::End(start.to_end())).unwrap();
                }
            }

            impl suds_util::xml::FromXml for #name {
                fn from_xml<R: std::io::BufRead>(reader: &mut suds_util::xml::Reader<R>, buffer: &mut Vec<u8>) -> Self {
                    suds_util::xml::expect_start(reader, buffer, #from_xml_name).unwrap();
                    let result = Self {
                        #(#from_xml_fields)*
                    };
                    suds_util::xml::expect_end(reader, buffer).unwrap();

                    result
                }
            }
        }
    }
}

impl Codegen for wsdl::Field {
    fn codegen(&self, _: &Namespaces) -> TokenStream {
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

fn codegen_to_xml_fields(fields: &[wsdl::Field], _: &Namespaces) -> Vec<TokenStream> {
    fields.iter().map(|field| {
        let name = format_ident!("{}", &field.name.name);
        let xml_name = format!("ns{}:{}", field.name.index(), &field.name.name);

        match &field.ty.name as &str {
            "int" => quote! { {
                let start = suds_util::xml::events::BytesStart::owned_name(#xml_name);
                let string = format!("{}", self.#name);
                let value = suds_util::xml::events::BytesText::from_plain_str(&string);
                writer.write_event(suds_util::xml::events::Event::Start(start.to_borrowed())).unwrap();
                writer.write_event(suds_util::xml::events::Event::Text(value)).unwrap();
                writer.write_event(suds_util::xml::events::Event::End(start.to_end())).unwrap();
            } },
            _ => quote!{ self.#name.to_xml(writer, top_level); }
        }
    }).collect()
}

fn codegen_from_xml_fields(fields: &[wsdl::Field], _: &Namespaces) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|field| {
            let name = format_ident!("{}", &field.name.name);
            let xml_name = &field.name.name;

            match &field.ty.name as &str {
                "int" => quote! { #name: {
                    suds_util::xml::expect_start(reader, buffer, #xml_name).unwrap();
                    let value = suds_util::xml::expect_value(reader, buffer).unwrap();
                    suds_util::xml::expect_end(reader, buffer).unwrap();

                    value
                }, },
                _ => {
                    let ident = format_ident!("{}", &field.ty.name);
                    quote! { #name: super::types::#ident::from_xml(reader, buffer), }
                }
            }
        })
        .collect()
}

impl Codegen for wsdl::Message {
    fn codegen(&self, namespaces: &Namespaces) -> TokenStream {
        let name = format_ident!("{}", &self.name.name);
        let fields = codegen_all(&self.parts, namespaces);

        let to_xml_fields = codegen_to_xml_fields(&self.parts, namespaces);
        let from_xml_fields = codegen_from_xml_fields(&self.parts, namespaces);

        quote! {
            #[derive(Debug, Clone)]
            pub struct #name {
                #(#fields)*
            }

            impl suds_util::xml::ToXml for #name {
                fn to_xml<W: std::io::Write>(&self, writer: &mut suds_util::xml::Writer<W>, top_level: bool) {
                    #(#to_xml_fields)*
                }
            }

            impl suds_util::xml::FromXml for #name {
                fn from_xml<R: std::io::BufRead>(reader: &mut suds_util::xml::Reader<R>, buffer: &mut Vec<u8>) -> Self {
                    Self {
                        #(#from_xml_fields)*
                    }
                }
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
                client: suds_util::soap::Client,
            }

            impl #name {
                pub fn new() -> Self {
                    Self {
                        client: suds_util::soap::Client::new(#location),
                    }
                }

                #(#operations)*
            }
        }
    }
}

impl Codegen for wsdl::Operation {
    fn codegen(&self, _: &Namespaces) -> TokenStream {
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
                let envelope = suds_util::soap::Envelope::new(input);
                self.client.send(envelope).into_body()
            }
        }
    }
}
