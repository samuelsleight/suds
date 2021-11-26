use super::types;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use suds_wsdl::types::{self as wsdl, Namespaces};

pub trait Codegen {
    fn codegen(&self, namespaces: &Namespaces) -> TokenStream;
}

fn get_ty_ident(ty: &str) -> Option<Ident> {
    match ty {
        "int" => Some(format_ident!("isize")),
        "unsignedShort" => Some(format_ident!("u16")),
        "unsignedInt" => Some(format_ident!("usize")),
        "string" => Some(format_ident!("String")),
        _ => None,
    }
}

fn codegen_all(all: &[impl Codegen], namespaces: &Namespaces) -> Vec<TokenStream> {
    all.iter().map(|item| item.codegen(namespaces)).collect()
}

impl Codegen for types::Definition {
    fn codegen(&self, namespaces: &Namespaces) -> TokenStream {
        let types = codegen_all(&self.types, namespaces);
        let messages = codegen_all(&self.messages, namespaces);
        let services = codegen_all(&self.services, namespaces);

        let namespace_attributes = namespaces
            .namespaces()
            .iter()
            .enumerate()
            .map(|(idx, url)| {
                let ns = format!("xmlns:ns{}", idx);
                quote! {.with_attributes([(#ns, #url)])}
            })
            .collect::<Vec<_>>();

        quote! {
            pub mod types {
                fn with_attributes<'a>(start: suds_util::xml::events::BytesStart<'a>) -> suds_util::xml::events::BytesStart<'a> {
                    start
                        #(#namespace_attributes)*
                }

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

        let to_xml_name = format!("ns{}:{}", self.name.index(), &self.name.name);
        let from_xml_name = &self.name.name;

        match &self.kind {
            wsdl::TypeKind::Simple(ty) => {
                let inner_ty = get_ty_ident(&ty.name).unwrap();

                quote! {
                    pub struct #name(pub #inner_ty);

                    impl suds_util::xml::ToXml for #name {
                        fn to_xml<W: std::io::Write>(&self, writer: &mut suds_util::xml::Writer<W>, mut top_level: bool) {
                            let start = suds_util::xml::events::BytesStart::owned_name(#to_xml_name);

                            let start = if top_level {
                                with_attributes(start)
                            } else {
                                start
                            };

                            top_level = false;

                            let string = format!("{}", 0);
                            let value = suds_util::xml::events::BytesText::from_plain_str(&string);

                            writer.write_event(suds_util::xml::events::Event::Start(start.to_borrowed())).unwrap();
                            writer.write_event(suds_util::xml::events::Event::Text(value)).unwrap();
                            writer.write_event(suds_util::xml::events::Event::End(start.to_end())).unwrap();
                        }
                    }

                    impl suds_util::xml::FromXml for #name {
                        fn from_xml<R: std::io::BufRead>(reader: &mut suds_util::xml::Reader<R>, buffer: &mut Vec<u8>) -> Self {
                            suds_util::xml::expect_start(reader, buffer, #from_xml_name).unwrap();
                            let value = suds_util::xml::expect_value(reader, buffer).unwrap();
                            suds_util::xml::expect_end(reader, buffer).unwrap();

                            Self(value)
                        }
                    }

                }
            }

            wsdl::TypeKind::Struct(fields) => {
                let member_fields = codegen_all(fields, namespaces);
                let to_xml_fields = codegen_to_xml_fields(fields, namespaces);
                let from_xml_fields = codegen_from_xml_fields(fields, namespaces);

                quote! {
                    #[derive(Debug, Clone)]
                    pub struct #name {
                        #(#member_fields)*
                    }

                    impl suds_util::xml::ToXml for #name {
                        fn to_xml<W: std::io::Write>(&self, writer: &mut suds_util::xml::Writer<W>, mut top_level: bool) {
                            let start = suds_util::xml::events::BytesStart::owned_name(#to_xml_name);

                            let start = if top_level {
                                with_attributes(start)
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

            wsdl::TypeKind::Alias(alias) => {
                if *alias != self.name {
                    let alias = format_ident!("{}", alias.name);
                    quote! {pub type #name = #alias;}
                } else {
                    quote! {}
                }
            }
        }
    }
}

impl Codegen for wsdl::Field {
    fn codegen(&self, namespaces: &Namespaces) -> TokenStream {
        let name = format_ident!("{}", &self.name.name);

        let ty = match &self.ty {
            wsdl::FieldKind::Type(name) => {
                if let Some(ident) = get_ty_ident(&name.name) {
                    quote! {#ident}
                } else {
                    let ident = format_ident!("{}", &name.name);
                    quote! { super::types::#ident }
                }
            }

            wsdl::FieldKind::Inner(wsdl::TypeKind::Struct(fields)) => {
                if fields.len() != 1 {
                    unimplemented!()
                }

                let mut field = fields.last().unwrap().clone();
                field.name = self.name.clone();
                return field.codegen(namespaces);
            }

            _ => unimplemented!(),
        };

        quote! {
            pub #name: #ty,
        }
    }
}

fn codegen_to_xml_field(field: &wsdl::Field) -> TokenStream {
    let name = format_ident!("{}", &field.name.name);
    let xml_name = format!("ns{}:{}", field.name.index(), &field.name.name);

    match &field.ty {
        wsdl::FieldKind::Type(ty) => match &ty.name as &str {
            "int" | "unsignedShort" | "string" => quote! { {
                let start = suds_util::xml::events::BytesStart::owned_name(#xml_name);
                let string = format!("{}", self.#name);
                let value = suds_util::xml::events::BytesText::from_plain_str(&string);
                writer.write_event(suds_util::xml::events::Event::Start(start.to_borrowed())).unwrap();
                writer.write_event(suds_util::xml::events::Event::Text(value)).unwrap();
                writer.write_event(suds_util::xml::events::Event::End(start.to_end())).unwrap();
            } },
            _ => quote! { self.#name.to_xml(writer, top_level); },
        },

        wsdl::FieldKind::Inner(wsdl::TypeKind::Struct(fields)) => {
            if fields.len() != 1 {
                unimplemented!()
            }

            let mut inner = fields.last().unwrap().clone();
            inner.name = field.name.clone();
            codegen_to_xml_field(&inner)
        }

        _ => unimplemented!(),
    }
}

fn codegen_to_xml_fields(fields: &[wsdl::Field], _: &Namespaces) -> Vec<TokenStream> {
    fields.iter().map(codegen_to_xml_field).collect()
}

fn codegen_from_xml_field(field: &wsdl::Field) -> TokenStream {
    let name = format_ident!("{}", &field.name.name);
    let xml_name = &field.name.name;

    match &field.ty {
        wsdl::FieldKind::Type(ty) => match &ty.name as &str {
            "int" | "unsignedShort" | "string" => quote! { #name: {
                suds_util::xml::expect_start(reader, buffer, #xml_name).unwrap();
                let value = suds_util::xml::expect_value(reader, buffer).unwrap();
                suds_util::xml::expect_end(reader, buffer).unwrap();

                value
            }, },
            _ => {
                let ident = format_ident!("{}", &ty.name);
                quote! { #name: super::types::#ident::from_xml(reader, buffer), }
            }
        },

        wsdl::FieldKind::Inner(wsdl::TypeKind::Struct(fields)) => {
            if fields.len() != 1 {
                unimplemented!()
            }

            let mut inner = fields.last().unwrap().clone();
            inner.name = field.name.clone();
            codegen_from_xml_field(&inner)
        }

        _ => unimplemented!(),
    }
}

fn codegen_from_xml_fields(fields: &[wsdl::Field], _: &Namespaces) -> Vec<TokenStream> {
    fields.iter().map(codegen_from_xml_field).collect()
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
