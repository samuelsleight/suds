use super::xml::{
    events::{BytesStart, Event},
    ToXml, Writer,
};

use reqwest::blocking::{Client as Reqwest, Response};

use std::io::{Cursor, Write};

pub struct Client {
    client: Reqwest,
    url: &'static str,
}

#[derive(Debug)]
pub struct Envelope<T: ToXml> {
    body: T,
}

impl Client {
    pub fn new(url: &'static str) -> Self {
        Self {
            client: Reqwest::new(),
            url,
        }
    }

    pub fn send<T: ToXml>(&self, envelope: Envelope<T>) -> Response {
        self.client
            .post(self.url)
            .body(envelope.body())
            .header(reqwest::header::CONTENT_TYPE, "text/xml")
            .send()
            .unwrap()
    }
}

impl<T: ToXml> Envelope<T> {
    pub fn new(body: T) -> Self {
        Self { body }
    }

    pub fn body(&self) -> Vec<u8> {
        let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);
        self.to_xml(&mut writer, true);
        writer.into_inner().into_inner()
    }
}

impl<T: ToXml> ToXml for Envelope<T> {
    fn to_xml<W: Write>(&self, writer: &mut Writer<W>, top_level: bool) {
        let envelope = BytesStart::owned_name("soapenv:Envelope")
            .with_attributes([("xmlns:soapenv", "http://schemas.xmlsoap.org/soap/envelope/")]);
        let body = BytesStart::owned_name("soapenv:Body");

        writer
            .write_event(Event::Start(envelope.to_borrowed()))
            .unwrap();
        writer
            .write_event(Event::Start(body.to_borrowed()))
            .unwrap();
        self.body.to_xml(writer, top_level);
        writer.write_event(Event::End(body.to_end())).unwrap();
        writer.write_event(Event::End(envelope.to_end())).unwrap();
    }
}
