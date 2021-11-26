use quick_xml::{
    events::{attributes::Attributes, BytesStart, BytesText, Event},
    Reader,
};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
};
use url::Url;

use crate::types::FieldKind;

use super::{
    error,
    types::{
        Binding, BindingOperation, Definition, Field, Message, NamespacedName, Namespaces,
        Operation, Port, PortType, Service, Type, TypeKind,
    },
};

fn get_attributes<B: BufRead, const N: usize>(
    reader: &Reader<B>,
    attributes: Attributes<'_>,
    names: [&'static str; N],
) -> Result<[Option<String>; N], error::Error> {
    const INIT: Option<String> = None;
    let mut result = [INIT; N];

    for attribute in attributes {
        let attribute = attribute?;
        let key = reader.decode(attribute.key)?;

        for (index, name) in names.iter().enumerate() {
            if key == *name {
                result[index] = Some(reader.decode(attribute.value.as_ref())?.to_owned());
                break;
            }
        }
    }

    Ok(result)
}

fn split_namespaced_name(prefixed_name: &str) -> (Option<&str>, &str) {
    let mut split = prefixed_name.split(':');
    let first = split.next().unwrap();
    let second = split.next();

    if let Some(second) = second {
        (Some(first), second)
    } else {
        (None, first)
    }
}

#[derive(Clone, Default)]
struct CurrentNamespaces {
    target: Vec<String>,
    namespaces: HashMap<Option<String>, String>,
}

struct Parser {
    root: Url,

    definition: Definition,
    namespaces: Namespaces,
    current_namespaces: CurrentNamespaces,
}

#[derive(Debug)]
enum ParseState {
    Definitions,

    Types,
    Schema,
    Element {
        name: String,
        kind: Option<TypeKind>,
    },
    ComplexType {
        name: Option<String>,
        kind: Option<TypeKind>,
    },
    ComplexContent {
        fields: Vec<Field>
    },
    ComplexExtension {
        fields: Vec<Field>
    },
    SimpleContent {
        ty: Option<NamespacedName>
    },
    SimpleExtension {
        ty: NamespacedName
    },
    Sequence(Vec<Field>),
    SequenceElement {
        name: String,
        ty: Option<NamespacedName>,
        inner: Option<TypeKind>,
    },
    SimpleType {
        name: String,
        ty: Option<NamespacedName>,
    },
    Restriction {
        ty: NamespacedName,
    },

    Message {
        name: String,
        parts: Vec<Field>,
    },
    Part {
        name: String,
        element: NamespacedName,
    },

    PortType {
        name: String,
        operations: Vec<Operation>,
    },
    Operation {
        name: String,
        documentation: Option<String>,
        input: Option<NamespacedName>,
        output: Option<NamespacedName>,
    },
    Documentation(Option<String>),
    Input {
        message: NamespacedName,
    },
    Output {
        message: NamespacedName,
    },

    Binding {
        name: String,
        ty: NamespacedName,
        transport: Option<String>,
        operations: Vec<BindingOperation>,
    },
    Transport {
        transport: String,
    },
    BindingOperation {
        name: String,
        action: Option<String>,
        style: Option<String>,
        input: Option<String>,
        output: Option<String>,
    },
    OperationAction {
        action: String,
        style: String,
    },
    BindingInput {
        body: Option<String>,
    },
    BindingOutput {
        body: Option<String>,
    },
    BindingBody {
        body: String,
    },

    Service {
        name: String,
        ports: Vec<Port>,
    },
    Port {
        name: String,
        binding: NamespacedName,
        address: Option<String>,
    },
    Address {
        location: String,
    },

    Import {
        namespace: Option<String>,
    },

    Other(String),
}

impl CurrentNamespaces {
    pub fn push_target_namespace(&mut self, namespace: String) {
        self.target.push(namespace);
    }

    pub fn pop_target_namespace(&mut self) {
        self.target.pop();
    }

    pub fn add_namespace_prefix(&mut self, prefix: Option<String>, namespace: &str) {
        self.namespaces.insert(prefix, namespace.to_owned());
    }

    pub fn target_namespaced(&self, namespaces: &mut Namespaces, name: String) -> NamespacedName {
        if let Some(target) = self.target.last() {
            NamespacedName::new(namespaces, target, name)
        } else {
            unimplemented!()
        }
    }

    pub fn resolved_prefix(
        &self,
        namespaces: &mut Namespaces,
        prefix: Option<String>,
        name: String,
    ) -> NamespacedName {
        match self.namespaces.get(&prefix) {
            Some(value) => NamespacedName::new(namespaces, value, name),
            None => unimplemented!(),
        }
    }
}

impl Parser {
    fn new(url: Url) -> Self {
        Self {
            root: url.clone(),

            definition: Default::default(),
            namespaces: Default::default(),
            current_namespaces: Default::default(),
        }
    }

    fn push_target_namespace(&mut self, namespace: String) {
        self.current_namespaces.push_target_namespace(namespace);
    }

    fn pop_target_namespace(&mut self) {
        self.current_namespaces.pop_target_namespace();
    }

    fn add_namespace_prefix(&mut self, prefix: Option<String>, namespace: &str) {
        self.current_namespaces
            .add_namespace_prefix(prefix, namespace);
    }

    fn target_namespaced(&mut self, name: String) -> NamespacedName {
        self.current_namespaces
            .target_namespaced(&mut self.namespaces, name)
    }

    fn resolved_prefix(&mut self, prefix: Option<String>, name: String) -> NamespacedName {
        self.current_namespaces
            .resolved_prefix(&mut self.namespaces, prefix, name)
    }

    fn resolve_namespace(&mut self, prefixed_name: &str) -> NamespacedName {
        let (prefix, local_name) = split_namespaced_name(prefixed_name);

        match prefix {
            Some("tns") => self.target_namespaced(local_name.to_owned()),

            _ => self.resolved_prefix(prefix.map(ToOwned::to_owned), local_name.to_owned()),
        }
    }

    fn parse(mut self) -> Result<(Definition, Namespaces), error::Error> {
        self.parse_url(self.root.clone())?;
        Ok((self.definition, self.namespaces))
    }

    fn parse_url(&mut self, url: Url) -> Result<(), error::Error> {
        println!("PARSING URL: {}", url);

        let result = match url.scheme() {
            "file" => self.parse_xml(
                url.clone(),
                Reader::from_file(
                    url.to_file_path()
                        .map_err(|()| error::Error::PathConversionError(None))?,
                )
                .map_err(error::Error::FileOpenError)?,
            ),

            "http" | "https" => self.parse_xml(url.clone(), Reader::from_reader(BufReader::new(
                reqwest::blocking::get(url)?,
            ))),

            other => Err(error::Error::UnsupportedScheme(other.into())),
        };

        println!("FINISHED PARSING FILE");
        result
    }

    fn parse_xml<B: BufRead>(&mut self, url: Url, mut reader: Reader<B>) -> Result<(), error::Error> {
        let mut stack = Vec::new();
        let mut buffer = Vec::new();
        let mut namespace_buffer = Vec::new();

        loop {
            let (namespace, event) =
                reader.read_namespaced_event(&mut buffer, &mut namespace_buffer)?;

            match event {
                Event::Decl(..) => (),

                Event::Start(start) => self.handle_start(&mut stack, &reader, start, namespace, &url)?,
                Event::End(..) => self.handle_end(&mut stack)?,

                Event::Empty(start) => {
                    self.handle_start(&mut stack, &reader, start, namespace, &url)?;
                    self.handle_end(&mut stack)?;
                }

                Event::Text(text) => self.handle_text(&mut stack, &reader, text)?,

                event => {
                    println!("{:?}", event);

                    if let Event::Eof = event {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_start<'a, B: BufRead>(
        &mut self,
        stack: &mut Vec<ParseState>,
        reader: &Reader<B>,
        start: BytesStart<'a>,
        namespace_bytes: Option<&[u8]>,
        url: &Url
    ) -> Result<(), error::Error> {
        let (prefix, local_name) = split_namespaced_name(reader.decode(start.name())?);

        let state = stack.pop();
        let mut new_state = Some(ParseState::Other(local_name.to_owned()));

        for attribute in start.attributes() {
            let attribute = attribute?;
            let key = reader.decode(attribute.key)?;
            let (prefix, value) = split_namespaced_name(key);

            if prefix == Some("xmlns") {
                self.add_namespace_prefix(
                    Some(value.to_owned()),
                    reader.decode(attribute.value.as_ref())?,
                );
            }
        }

        match state {
            None => match local_name {
                "definitions" => {
                    let [namespace] =
                        get_attributes(reader, start.attributes(), ["targetNamespace"])?;

                    if let Some(namespace) = namespace {
                        self.push_target_namespace(namespace)
                    } else {
                        unimplemented!()
                    }

                    new_state = Some(ParseState::Definitions)
                }

                "schema" => {
                    let [namespace] =
                        get_attributes(reader, start.attributes(), ["targetNamespace"])?;

                    if let Some(namespace) = namespace {
                        self.push_target_namespace(namespace);
                        self.add_namespace_prefix(
                            prefix.map(ToOwned::to_owned),
                            namespace_bytes
                                .and_then(|ns| std::str::from_utf8(ns).ok())
                                .unwrap(),
                        );
                    } else {
                        unimplemented!()
                    };

                    new_state = Some(ParseState::Schema)
                }

                _ => (),
            },

            Some(ParseState::Definitions) => match local_name {
                "import" => {
                    let [location, namespace] =
                        get_attributes(reader, start.attributes(), ["location", "namespace"])?;

                    let location = if let Some(location) = location {
                        location
                    } else {
                        unimplemented!()
                    };

                    self.parse_url(self.root.join(&location)?)?;
                    println!("BACK TO {}", url);

                    new_state = Some(ParseState::Import { namespace });
                }

                "types" => new_state = Some(ParseState::Types),

                "message" => {
                    let [name] = get_attributes(reader, start.attributes(), ["name"])?;

                    let name = if let Some(name) = name {
                        name
                    } else {
                        unimplemented!()
                    };

                    new_state = Some(ParseState::Message {
                        name,
                        parts: Vec::new(),
                    });
                }

                "portType" => {
                    let [name] = get_attributes(reader, start.attributes(), ["name"])?;

                    let name = if let Some(name) = name {
                        name
                    } else {
                        unimplemented!()
                    };

                    new_state = Some(ParseState::PortType {
                        name,
                        operations: Vec::new(),
                    });
                }

                "binding" => {
                    let [name, ty] = get_attributes(reader, start.attributes(), ["name", "type"])?;

                    let name = if let Some(name) = name {
                        name
                    } else {
                        unimplemented!()
                    };

                    let ty = if let Some(ty) = ty {
                        self.resolve_namespace(&ty)
                    } else {
                        unimplemented!()
                    };

                    new_state = Some(ParseState::Binding {
                        name,
                        ty,
                        transport: None,
                        operations: Vec::new(),
                    });
                }

                "service" => {
                    let [name] = get_attributes(reader, start.attributes(), ["name"])?;

                    let name = if let Some(name) = name {
                        name
                    } else {
                        unimplemented!()
                    };

                    new_state = Some(ParseState::Service {
                        name,
                        ports: Vec::new(),
                    });
                }

                _ => println!("FOUND {} INSIDE DEFINITION BLOCK", local_name),
            },

            Some(ParseState::Types) => match local_name {
                "schema" => {
                    let [namespace] =
                        get_attributes(reader, start.attributes(), ["targetNamespace"])?;

                    if let Some(namespace) = namespace {
                        self.push_target_namespace(namespace);
                        self.add_namespace_prefix(
                            prefix.map(ToOwned::to_owned),
                            namespace_bytes
                                .and_then(|ns| std::str::from_utf8(ns).ok())
                                .unwrap(),
                        );
                    } else {
                        unimplemented!()
                    };

                    new_state = Some(ParseState::Schema)
                }

                "import" => {
                    let [location, namespace] = get_attributes(
                        reader,
                        start.attributes(),
                        ["schemaLocation", "namespace"],
                    )?;

                    let location = if let Some(location) = location {
                        location
                    } else {
                        unimplemented!()
                    };

                    self.parse_url(self.root.join(&location)?)?;
                    println!("BACK TO {}", url);

                    new_state = Some(ParseState::Import { namespace });
                }

                _ => println!("FOUND {} INSIDE TYPES BLOCK", local_name),
            },

            Some(ParseState::Schema { .. }) => match local_name {
                "element" => {
                    let [name, ty] = get_attributes(reader, start.attributes(), ["name", "type"])?;

                    let name = if let Some(name) = name {
                        name
                    } else {
                        unimplemented!()
                    };

                    let kind = if let Some(ty) = ty {
                        Some(TypeKind::Alias(self.resolve_namespace(&ty)))
                    } else {
                        None
                    };

                    new_state = Some(ParseState::Element { name, kind })
                }

                "complexType" => {
                    let [name] = get_attributes(reader, start.attributes(), ["name"])?;

                    let name = if let Some(name) = name {
                        name
                    } else {
                        unimplemented!()
                    };

                    new_state = Some(ParseState::ComplexType {
                        kind: None,
                        name: Some(name),
                    });
                }

                "simpleType" => {
                    let [name] = get_attributes(reader, start.attributes(), ["name"])?;

                    let name = if let Some(name) = name {
                        name
                    } else {
                        unimplemented!()
                    };

                    new_state = Some(ParseState::SimpleType { name, ty: None })
                }

                "include" | "import" => {
                    let [location, namespace] = get_attributes(
                        reader,
                        start.attributes(),
                        ["schemaLocation", "namespace"],
                    )?;

                    let location = if let Some(location) = location {
                        location
                    } else {
                        unimplemented!()
                    };

                    self.parse_url(self.root.join(&location)?)?;
                    println!("BACK TO {}", url);

                    new_state = Some(ParseState::Import { namespace });
                }

                _ => println!("FOUND {} INSIDE SCHEMA BLOCK", local_name),
            },

            Some(ParseState::Element { .. }) => match local_name {
                "complexType" => {
                    new_state = Some(ParseState::ComplexType {
                        kind: None,
                        name: None,
                    })
                }

                _ => println!("FOUND {} INSIDE ELEMENT BLOCK", local_name),
            },

            Some(ParseState::ComplexType { .. }) => match local_name {
                "sequence" => new_state = Some(ParseState::Sequence(Vec::new())),

                "simpleContent" => new_state = Some(ParseState::SimpleContent{ty: None}),

                "complexContent" => new_state = Some(ParseState::ComplexContent{fields: Vec::new()}),

                _ => println!("FOUND {} INSIDE COMPLEX TYPE BLOCK", local_name),
            },

            Some(ParseState::ComplexContent { .. }) => match local_name {
                "extension" => {
                    let [base] = get_attributes(reader, start.attributes(), ["base"])?;

                    let ty = if let Some(base) = base {
                        self.resolve_namespace(&base)
                    } else {
                        unimplemented!()
                    };

                    let field = Field {
                        name: self.resolve_namespace("tns:base"),
                        ty: FieldKind::Type(ty)
                    };

                    new_state = Some(ParseState::ComplexExtension { fields: vec![field] });
                },

                _ => println!("FOUND {} INSIDE COMPLEX CONTENT BLOCK", local_name),
            },

            Some(ParseState::ComplexExtension { .. }) => match local_name {
                "sequence" => new_state = Some(ParseState::Sequence(Vec::new())),

                _ => println!("FOUND {} INSIDE COMPLEX EXTENSION BLOCK", local_name),
            }

            Some(ParseState::SimpleExtension { .. }) => println!("FOUND {} INSIDE SIMPLE EXTENSION BLOCK", local_name),

            Some(ParseState::SimpleContent { .. }) => match local_name {
                "extension" => {
                    let [base] = get_attributes(reader, start.attributes(), ["base"])?;

                    let ty = if let Some(base) = base {
                        self.resolve_namespace(&base)
                    } else {
                        unimplemented!()
                    };

                    new_state = Some(ParseState::SimpleExtension { ty });
                },

                _ => println!("FOUND {} INSIDE SIMPLE CONTENT BLOCK", local_name),
            },

            Some(ParseState::SimpleType { .. }) => match local_name {
                "restriction" => {
                    let [base] = get_attributes(reader, start.attributes(), ["base"])?;

                    let ty = if let Some(base) = base {
                        self.resolve_namespace(&base)
                    } else {
                        unimplemented!()
                    };

                    new_state = Some(ParseState::Restriction { ty });
                }

                _ => println!("FOUND {} INSIDE SIMPLE TYPE BLOCK", local_name),
            },

            Some(ParseState::Restriction { .. }) => {
                println!("FOUND {} INSIDE RESTRICTION BLOCK", local_name)
            }

            Some(ParseState::Sequence(_)) => match local_name {
                "element" => {
                    let [name, ty] = get_attributes(reader, start.attributes(), ["name", "type"])?;

                    let name = if let Some(name) = name {
                        name
                    } else {
                        unimplemented!()
                    };

                    let ty = if let Some(ty) = ty {
                        Some(self.resolve_namespace(&ty))
                    } else {
                        println!("{:?}", start);
                        None
                    };

                    new_state = Some(ParseState::SequenceElement {
                        name,
                        ty,
                        inner: None,
                    });
                }

                _ => println!("FOUND {} INSIDE SEQUENCE BLOCK", local_name),
            },

            Some(ParseState::SequenceElement { .. }) => match local_name {
                "complexType" => {
                    new_state = Some(ParseState::ComplexType {
                        kind: None,
                        name: None,
                    })
                }

                _ => println!("FOUND {} INSIDE SEQUENCE ELEMENT BLOCK", local_name),
            },

            Some(ParseState::Message { .. }) => match local_name {
                "part" => {
                    let [name, element] =
                        get_attributes(reader, start.attributes(), ["name", "element"])?;

                    let name = if let Some(name) = name {
                        name
                    } else {
                        unimplemented!()
                    };

                    let element = if let Some(element) = element {
                        self.resolve_namespace(&element)
                    } else {
                        unimplemented!()
                    };

                    new_state = Some(ParseState::Part { name, element });
                }

                _ => println!("FOUND {} INSIDE MESSAGE BLOCK", local_name),
            },

            Some(ParseState::Part { .. }) => match local_name {
                _ => println!("FOUND {} INSIDE MESSAGE PATH BLOCK", local_name),
            },

            Some(ParseState::PortType { .. }) => match local_name {
                "operation" => {
                    let [name] = get_attributes(reader, start.attributes(), ["name"])?;

                    let name = if let Some(name) = name {
                        name
                    } else {
                        unimplemented!()
                    };

                    new_state = Some(ParseState::Operation {
                        name,
                        documentation: None,
                        input: None,
                        output: None,
                    })
                }

                _ => println!("FOUND {} INSIDE PORT TYPE BLOCK", local_name),
            },

            Some(ParseState::Operation { .. }) => match local_name {
                "documentation" => new_state = Some(ParseState::Documentation(None)),

                "input" | "output" => {
                    let [message] = get_attributes(reader, start.attributes(), ["message"])?;

                    let message = if let Some(message) = message {
                        self.resolve_namespace(&message)
                    } else {
                        unimplemented!()
                    };

                    if local_name == "input" {
                        new_state = Some(ParseState::Input { message })
                    } else {
                        new_state = Some(ParseState::Output { message })
                    }
                }

                _ => println!("FOUND {} INSIDE OPERATION BLOCK", local_name),
            },

            Some(ParseState::Documentation(_)) => match local_name {
                _ => println!("FOUND {} INSIDE DOCUMENTATION BLOCK", local_name),
            },

            Some(ParseState::Input { .. }) => match local_name {
                _ => println!("FOUND {} INSIDE INPUT BLOCK", local_name),
            },

            Some(ParseState::Output { .. }) => match local_name {
                _ => println!("FOUND {} INSIDE OUTPUT BLOCK", local_name),
            },

            Some(ParseState::Binding { .. }) => match local_name {
                "binding" => {
                    let [transport] = get_attributes(reader, start.attributes(), ["transport"])?;

                    let transport = if let Some(transport) = transport {
                        transport
                    } else {
                        unimplemented!()
                    };

                    new_state = Some(ParseState::Transport { transport })
                }

                "operation" => {
                    let [name] = get_attributes(reader, start.attributes(), ["name"])?;

                    let name = if let Some(name) = name {
                        name
                    } else {
                        unimplemented!()
                    };

                    new_state = Some(ParseState::BindingOperation {
                        name,
                        action: None,
                        style: None,
                        input: None,
                        output: None,
                    })
                }

                _ => println!("FOUND {} INSIDE BINDING BLOCK", local_name),
            },

            Some(ParseState::Transport { .. }) => match local_name {
                _ => println!("FOUND {} INSIDE TRANSPORT BLOCK", local_name),
            },

            Some(ParseState::BindingOperation { .. }) => match local_name {
                "operation" => {
                    let [action, style] =
                        get_attributes(reader, start.attributes(), ["soapAction", "style"])?;

                    let action = if let Some(action) = action {
                        action
                    } else {
                        unimplemented!()
                    };

                    let style = if let Some(style) = style {
                        style
                    } else {
                        unimplemented!()
                    };

                    new_state = Some(ParseState::OperationAction { action, style });
                }

                "input" => new_state = Some(ParseState::BindingInput { body: None }),
                "output" => new_state = Some(ParseState::BindingOutput { body: None }),

                _ => println!("FOUND {} INSIDE BINDING OPERATION BLOCK", local_name),
            },

            Some(ParseState::OperationAction { .. }) => match local_name {
                _ => println!("FOUND {} INSIDE OPERATION ACTION BLOCK", local_name),
            },

            Some(ParseState::BindingInput { .. } | ParseState::BindingOutput { .. }) => {
                match local_name {
                    "body" => {
                        let [body] = get_attributes(reader, start.attributes(), ["use"])?;

                        let body = if let Some(body) = body {
                            body
                        } else {
                            unimplemented!()
                        };

                        new_state = Some(ParseState::BindingBody { body });
                    }

                    _ => println!("FOUND {} INSIDE OPERATION ACTION BLOCK", local_name),
                }
            }

            Some(ParseState::BindingBody { .. }) => match local_name {
                _ => println!("FOUND {} INSIDE OPERATION ACTION BLOCK", local_name),
            },

            Some(ParseState::Service { .. }) => match local_name {
                "port" => {
                    let [name, binding] =
                        get_attributes(reader, start.attributes(), ["name", "binding"])?;

                    let name = if let Some(name) = name {
                        name
                    } else {
                        unimplemented!()
                    };

                    let binding = if let Some(binding) = binding {
                        self.resolve_namespace(&binding)
                    } else {
                        unimplemented!()
                    };

                    new_state = Some(ParseState::Port {
                        name,
                        binding,
                        address: None,
                    });
                }

                _ => println!("FOUND {} INSIDE SERVICE BLOCK", local_name),
            },

            Some(ParseState::Port { .. }) => match local_name {
                "address" => {
                    let [location] = get_attributes(reader, start.attributes(), ["location"])?;

                    let location = if let Some(location) = location {
                        location
                    } else {
                        unimplemented!()
                    };

                    new_state = Some(ParseState::Address { location })
                }

                _ => println!("FOUND {} INSIDE PORT BLOCK", local_name),
            },

            Some(ParseState::Address { .. }) => match local_name {
                _ => println!("FOUND {} INSIDE LOCATION BLOCK", local_name),
            },

            Some(ParseState::Import { .. }) => unimplemented!(),

            Some(ParseState::Other(ref name)) => {
                println!("FOUND {} INSIDE {} BLOCK", local_name, name);
            }
        }

        stack.extend(state);
        stack.extend(new_state);

        Ok(())
    }

    fn handle_end(&mut self, stack: &mut Vec<ParseState>) -> Result<(), error::Error> {
        let finished_state = stack.pop();
        let mut next_state = stack.pop();

        match finished_state {
            Some(ParseState::Definitions | ParseState::Schema) => self.pop_target_namespace(),

            Some(ParseState::Element { name, kind }) => {
                let kind = if let Some(kind) = kind {
                    kind
                } else {
                    unimplemented!()
                };

                let name = self.target_namespaced(name);
                self.definition.types.push(Type { name, kind })
            }

            Some(ParseState::ComplexType { kind, name }) => match next_state {
                Some(ParseState::SequenceElement {
                    ref mut ty,
                    ref mut inner,
                    ..
                }) => {
                    *ty = name.map(|name| self.target_namespaced(name));
                    *inner = kind;
                }

                Some(ParseState::Element {
                    kind: ref mut el_kind,
                    ..
                }) => {
                    if name.is_some() {
                        unimplemented!()
                    }

                    *el_kind = kind;
                }

                _ => {
                    let kind = if let Some(kind) = kind {
                        kind
                    } else {
                        unimplemented!()
                    };

                    let name = if let Some(name) = name {
                        self.target_namespaced(name)
                    } else {
                        unimplemented!()
                    };

                    self.definition.types.push(Type { name, kind })
                }
            },

            Some(ParseState::ComplexContent { fields }) => match next_state {
                Some(ParseState::ComplexType { ref mut kind, .. }) if kind.is_none() => {
                    *kind = Some(TypeKind::Struct(fields))
                },

                _ => unimplemented!()
            }

            Some(ParseState::ComplexExtension { fields }) => match next_state {
                Some(ParseState::ComplexContent { fields: ref mut content  }) => content.extend(fields.into_iter()),

                _ => unimplemented!()
            }

            Some(ParseState::SimpleContent { ty}) => match next_state {
                Some(ParseState::ComplexType { ref mut kind, .. }) if kind.is_none() => {
                    *kind = Some(TypeKind::Alias(ty.unwrap()))
                },

                _ => unimplemented!()
            }

            Some(ParseState::SimpleExtension { ty: base }) => match next_state {
                Some(ParseState::SimpleContent { ref mut ty }) => *ty = Some(base),

                _ => unimplemented!()
            }

            Some(ParseState::SimpleType { name, ty }) => {
                let kind = if let Some(ty) = ty {
                    TypeKind::Simple(ty)
                } else {
                    unimplemented!()
                };

                let name = self.target_namespaced(name);
                self.definition.types.push(Type { name, kind })
            }

            Some(ParseState::Restriction { ty: base }) => match next_state {
                Some(ParseState::SimpleType { ref mut ty, .. }) => *ty = Some(base),
                _ => unimplemented!(),
            },

            Some(ParseState::Sequence(fields)) => match next_state {
                Some(ParseState::ComplexType { ref mut kind, .. }) if kind.is_none() => {
                    *kind = Some(TypeKind::Struct(fields))
                },

                Some(ParseState::ComplexExtension { fields: ref mut extension_fields, .. }) => {
                    extension_fields.extend(fields.into_iter())
                },

                _ => unimplemented!(),
            },

            Some(ParseState::SequenceElement { name, ty, inner }) => match next_state {
                Some(ParseState::Sequence(ref mut fields)) => fields.push(Field {
                    name: self.target_namespaced(name),
                    ty: if let Some(kind) = inner {
                        FieldKind::Inner(kind)
                    } else {
                        FieldKind::Type(ty.unwrap())
                    },
                }),
                _ => unimplemented!(),
            },

            Some(ParseState::Message { name, parts }) => {
                let name = self.target_namespaced(name);
                self.definition.messages.push(Message { name, parts })
            }

            Some(ParseState::Part { name, element }) => match next_state {
                Some(ParseState::Message { ref mut parts, .. }) => parts.push(Field {
                    name: self.target_namespaced(name),
                    ty: FieldKind::Type(element),
                }),
                _ => unimplemented!(),
            },

            Some(ParseState::PortType { name, operations }) => {
                let name = self.target_namespaced(name);
                self.definition
                    .port_types
                    .push(PortType { name, operations })
            }

            Some(ParseState::Operation {
                name,
                input,
                output,
                documentation,
            }) => match next_state {
                Some(ParseState::PortType {
                    ref mut operations, ..
                }) => operations.push(Operation {
                    name: self.target_namespaced(name),
                    input,
                    output,
                    documentation,
                }),
                _ => unimplemented!(),
            },

            Some(ParseState::Documentation(text)) => match next_state {
                Some(ParseState::Operation {
                    ref mut documentation,
                    ..
                }) => *documentation = text,
                _ => unimplemented!(),
            },

            Some(ParseState::Input { message }) => match next_state {
                Some(ParseState::Operation { ref mut input, .. }) if input.is_none() => {
                    *input = Some(message)
                }
                _ => unimplemented!(),
            },

            Some(ParseState::Output { message }) => match next_state {
                Some(ParseState::Operation { ref mut output, .. }) if output.is_none() => {
                    *output = Some(message)
                }
                _ => unimplemented!(),
            },

            Some(ParseState::Transport { transport: kind }) => match next_state {
                Some(ParseState::Binding {
                    ref mut transport, ..
                }) if transport.is_none() => *transport = Some(kind),
                _ => unimplemented!(),
            },

            Some(ParseState::Binding {
                name,
                ty,
                transport,
                operations,
            }) => {
                let name = self.target_namespaced(name);
                self.definition.bindings.push(Binding {
                    name,
                    ty,
                    transport: transport.unwrap(),
                    operations,
                })
            }

            Some(ParseState::BindingOperation {
                name,
                action,
                style,
                input,
                output,
            }) => match next_state {
                Some(ParseState::Binding {
                    ref mut operations, ..
                }) => operations.push(BindingOperation {
                    name: self.target_namespaced(name),
                    action: action.unwrap(),
                    style: style.unwrap(),
                    input,
                    output,
                }),
                _ => unimplemented!(),
            },

            Some(ParseState::OperationAction { action, style }) => match next_state {
                Some(ParseState::BindingOperation {
                    action: ref mut a,
                    style: ref mut s,
                    ..
                }) => {
                    *a = Some(action);
                    *s = Some(style);
                }
                _ => unimplemented!(),
            },

            Some(ParseState::BindingInput { body }) => match next_state {
                Some(ParseState::BindingOperation { ref mut input, .. }) => *input = body,
                _ => unimplemented!(),
            },

            Some(ParseState::BindingOutput { body }) => match next_state {
                Some(ParseState::BindingOperation { ref mut output, .. }) => *output = body,
                _ => unimplemented!(),
            },

            Some(ParseState::BindingBody { body: body_use }) => match next_state {
                Some(
                    ParseState::BindingInput { ref mut body }
                    | ParseState::BindingOutput { ref mut body },
                ) => *body = Some(body_use),
                _ => unimplemented!(),
            },

            Some(ParseState::Service { name, ports }) => {
                let name = self.target_namespaced(name);
                self.definition.services.push(Service { name, ports })
            }

            Some(ParseState::Port {
                name,
                binding,
                address,
            }) => match next_state {
                Some(ParseState::Service { ref mut ports, .. }) => ports.push(Port {
                    name: self.target_namespaced(name),
                    binding,
                    location: address.unwrap(),
                }),
                _ => unimplemented!(),
            },

            Some(ParseState::Address { location }) => match next_state {
                Some(ParseState::Port {
                    ref mut address, ..
                }) => *address = Some(location),
                _ => unimplemented!(),
            },

            _ => (),
        }

        stack.extend(next_state);
        Ok(())
    }

    fn handle_text<'a, B: BufRead>(
        &mut self,
        stack: &mut Vec<ParseState>,
        reader: &Reader<B>,
        start: BytesText<'a>,
    ) -> Result<(), error::Error> {
        let unescaped = start.unescaped()?;
        let text = reader.decode(unescaped.as_ref())?;
        let mut state = stack.pop();

        match state {
            Some(ParseState::Documentation(ref mut docs)) => *docs = Some(text.to_owned()),
            _ => (),
        }

        stack.extend(state);
        Ok(())
    }
}

pub fn parse(url: Url) -> Result<(Definition, Namespaces), error::Error> {
    Parser::new(url).parse()
}
