use std::path::Path;
use url::Url;

mod error;
mod parser;

#[derive(Debug, Clone)]
pub enum TypeKind {
    Struct(Vec<Field>),
}

#[derive(Debug, Clone)]
pub struct Type {
    pub name: String,
    pub kind: TypeKind,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub name: String,
    pub parts: Vec<Field>,
}

#[derive(Debug, Clone)]
pub struct Operation {
    pub name: String,
    pub documentation: Option<String>,
    pub input: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PortType {
    pub name: String,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone)]
pub struct BindingOperation {
    pub name: String,
    pub action: String,
    pub style: String,
    pub input: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Binding {
    pub name: String,
    pub ty: String,
    pub transport: String,
    pub operations: Vec<BindingOperation>,
}

#[derive(Debug, Clone)]
pub struct Port {
    pub name: String,
    pub binding: String,
    pub location: String,
}

#[derive(Debug, Clone)]
pub struct Service {
    pub name: String,
    pub ports: Vec<Port>,
}

#[derive(Default, Debug, Clone)]
pub struct Definition {
    pub types: Vec<Type>,
    pub messages: Vec<Message>,
    pub port_types: Vec<PortType>,
    pub bindings: Vec<Binding>,
    pub services: Vec<Service>,
}

pub fn parse<S: AsRef<str>>(url: S) -> Result<Definition, error::Error> {
    let url = {
        match Url::parse(url.as_ref()) {
            Ok(url) => url,
            Err(url::ParseError::RelativeUrlWithoutBase) => Url::from_file_path(
                &Path::new(url.as_ref())
                    .canonicalize()
                    .map_err(|err| error::Error::PathConversionError(Some(err)))?,
            )
            .unwrap(),
            Err(err) => return Err(err.into()),
        }
    };

    parser::parse(url)
}
