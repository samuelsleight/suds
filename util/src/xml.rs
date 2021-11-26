use std::io::Write;

pub use quick_xml::{events, Writer};

pub trait ToXml {
    fn to_xml<W: Write>(&self, writer: &mut Writer<W>, top_level: bool);
}
