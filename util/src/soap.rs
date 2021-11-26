use super::xml::{
    events::{BytesStart, Event},
    expect_end, expect_start, FromXml, Reader, ToXml, Writer,
};

use bytes::Buf;
use reqwest::blocking::Client as Reqwest;
use std::io::{BufRead, BufReader, Cursor, Read, Write};

pub struct Client {
    client: Reqwest,
    url: &'static str,
}

#[derive(Debug)]
pub struct Envelope<T> {
    body: T,
}

impl Client {
    pub fn new(url: &'static str) -> Self {
        Self {
            client: Reqwest::new(),
            url,
        }
    }

    pub fn send<T: ToXml, U: FromXml>(&self, request_envelope: Envelope<T>) -> Envelope<U> {
        let response = self
            .client
            .post(self.url)
            .body(request_envelope.to_request())
            .header(reqwest::header::CONTENT_TYPE, "text/xml")
            .send()
            .unwrap();

        Envelope::<U>::from_response(response.bytes().unwrap().reader())
    }
}

impl<T> Envelope<T> {
    pub fn new(body: T) -> Self {
        Self { body }
    }

    pub fn into_body(self) -> T {
        self.body
    }
}

impl<T: ToXml> Envelope<T> {
    pub fn to_request(&self) -> Vec<u8> {
        let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);
        self.to_xml(&mut writer, true);
        writer.into_inner().into_inner()
    }
}

impl<T: FromXml> Envelope<T> {
    pub fn from_response<R: Read>(read: R) -> Self {
        let mut reader = Reader::from_reader(BufReader::new(read));
        reader.trim_text(true);
        reader.expand_empty_elements(true);
        let mut buffer = Vec::new();
        Self::from_xml(&mut reader, &mut buffer)
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

impl<T: FromXml> FromXml for Envelope<T> {
    fn from_xml<R: BufRead>(reader: &mut Reader<R>, buffer: &mut Vec<u8>) -> Self {
        expect_start(reader, buffer, "Envelope").unwrap();
        expect_start(reader, buffer, "Body").unwrap();
        let body = T::from_xml(reader, buffer);
        expect_end(reader, buffer).unwrap();
        expect_end(reader, buffer).unwrap();

        Self::new(body)
    }
}
