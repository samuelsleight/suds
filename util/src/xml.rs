use std::{
    fmt::Debug,
    io::{BufRead, Write},
    str::FromStr,
};

pub use quick_xml::{events, Reader, Writer};

pub trait ToXml {
    fn to_xml<W: Write>(&self, writer: &mut Writer<W>, top_level: bool);
}

pub trait FromXml {
    fn from_xml<R: BufRead>(reader: &mut Reader<R>, buffer: &mut Vec<u8>) -> Self;
}

fn next_event<R: BufRead>(
    reader: &mut Reader<R>,
    buffer: &mut Vec<u8>,
) -> Option<events::Event<'static>> {
    loop {
        match reader.read_event(buffer).unwrap() {
            event
            @
            (events::Event::Start(_)
            | events::Event::Empty(_)
            | events::Event::End(_)
            | events::Event::Text(_)) => break Some(event.into_owned()),
            events::Event::Eof => return None,
            _ => (),
        }
    }
}

pub fn is_start<'a>(event: events::Event<'a>, name: &str) -> Option<events::BytesStart<'a>> {
    if let events::Event::Start(start) = event {
        if start.local_name() == name.as_bytes() {
            return Some(start);
        }
    }

    None
}

pub fn expect_start<'a, R: BufRead>(
    reader: &mut Reader<R>,
    buffer: &'a mut Vec<u8>,
    name: &str,
) -> Option<events::BytesStart<'a>> {
    is_start(next_event(reader, buffer).unwrap(), name)
}

pub fn expect_value<'a, R: BufRead, T: FromStr>(
    reader: &'a mut Reader<R>,
    buffer: &'a mut Vec<u8>,
) -> Option<T>
where
    <T as FromStr>::Err: Debug,
{
    if let Ok(events::Event::Text(text)) = reader.read_event(buffer) {
        let unescaped = text.unescaped().unwrap();
        let text = reader.decode(unescaped.as_ref()).unwrap();
        return Some(text.parse().unwrap());
    }

    None
}

pub fn expect_end<'a, R: BufRead>(
    reader: &'a mut Reader<R>,
    buffer: &'a mut Vec<u8>,
) -> Option<events::BytesEnd<'a>> {
    if let Ok(events::Event::End(end)) = reader.read_event(buffer) {
        return Some(end);
    }

    None
}
