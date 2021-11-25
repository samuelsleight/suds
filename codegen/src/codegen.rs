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

fn codegen_soap(namespaces: &Namespaces) -> TokenStream {
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
        mod soap {
            pub trait ToXml {
                fn to_xml<W: std::io::Write>(&self, writer: &mut quick_xml::Writer<W>);
            }

            #[derive(Debug)]
            pub struct Envelope<T: ToXml> {
                body: T
            }

            impl<T: ToXml> Envelope<T> {
                pub fn new(body: T) -> Self {
                    Self {
                        body
                    }
                }
            }

            impl<T: ToXml> ToXml for Envelope<T> {
                fn to_xml<W: std::io::Write>(&self, writer: &mut quick_xml::Writer<W>) {
                    let envelope = quick_xml::events::BytesStart::owned_name("soapenv:Envelope")
                        .with_attributes([("xmlns:soapenv", "http://schemas.xmlsoap.org/soap/envelope/")])
                        #(#namespaces)*;
                    let body = quick_xml::events::BytesStart::owned_name("soapenv:Body");

                    writer.write_event(quick_xml::events::Event::Start(envelope.to_borrowed())).unwrap();
                    writer.write_event(quick_xml::events::Event::Start(body.to_borrowed())).unwrap();
                    self.body.to_xml(writer);
                    writer.write_event(quick_xml::events::Event::End(body.to_end())).unwrap();
                    writer.write_event(quick_xml::events::Event::End(envelope.to_end())).unwrap();
                }
            }
        }
    }
}

impl Codegen for types::Definition {
    fn codegen(&self, namespaces: &Namespaces) -> TokenStream {
        let soap = codegen_soap(namespaces);
        let types = codegen_all(&self.types, namespaces);
        let messages = codegen_all(&self.messages, namespaces);
        let services = codegen_all(&self.services, namespaces);

        quote! {
            #soap

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
        let (fields, xml_fields) = match &self.kind {
            wsdl::TypeKind::Struct(fields) => (
                codegen_all(fields, namespaces),
                codegen_fields(fields, namespaces),
            ),
        };

        let xml_name = format!("ns{}:{}", self.name.index(), &self.name.name);

        quote! {
            #[derive(Debug, Clone)]
            pub struct #name {
                #(#fields)*
            }

            impl super::soap::ToXml for #name {
                fn to_xml<W: std::io::Write>(&self, writer: &mut quick_xml::Writer<W>) {
                    let start = quick_xml::events::BytesStart::owned_name(#xml_name);
                    writer.write_event(quick_xml::events::Event::Start(start.to_borrowed())).unwrap();
                    #(#xml_fields)*
                    writer.write_event(quick_xml::events::Event::End(start.to_end())).unwrap();
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

fn codegen_fields(fields: &[wsdl::Field], _: &Namespaces) -> Vec<TokenStream> {
    fields.iter().map(|field| {
        let name = format_ident!("{}", &field.name.name);
        let xml_name = format!("ns{}:{}", field.name.index(), &field.name.name);

        match &field.ty.name as &str {
            "int" => quote! { {
                let start = quick_xml::events::BytesStart::owned_name(#xml_name);
                let string = format!("{}", self.#name);
                let value = quick_xml::events::BytesText::from_plain_str(&string);
                writer.write_event(quick_xml::events::Event::Start(start.to_borrowed())).unwrap();
                writer.write_event(quick_xml::events::Event::Text(value)).unwrap();
                writer.write_event(quick_xml::events::Event::End(start.to_end())).unwrap();
            } },
            _ => quote!{ self.#name.to_xml(writer); }
        }
    }).collect()
}

impl Codegen for wsdl::Message {
    fn codegen(&self, namespaces: &Namespaces) -> TokenStream {
        let name = format_ident!("{}", &self.name.name);
        let fields = codegen_all(&self.parts, namespaces);

        let xml_fields = codegen_fields(&self.parts, namespaces);

        quote! {
            #[derive(Debug, Clone)]
            pub struct #name {
                #(#fields)*
            }

            impl super::soap::ToXml for #name {
                fn to_xml<W: std::io::Write>(&self, writer: &mut quick_xml::Writer<W>) {
                    #(#xml_fields)*
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
                let envelope = super::super::soap::Envelope::new(input);
                println!("{:?}", envelope);

                let mut writer = quick_xml::Writer::new_with_indent(std::io::Cursor::new(Vec::new()), b' ', 2);
                super::super::soap::ToXml::to_xml(&envelope, &mut writer);
                println!("{}", String::from_utf8(writer.into_inner().into_inner()).unwrap());
                unimplemented!()
            }
        }
    }
}
