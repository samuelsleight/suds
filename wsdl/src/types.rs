#[derive(Default, Debug, Clone)]
pub struct Namespaces(Vec<String>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespacedName {
    namespace_idx: usize,
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    Struct(Vec<Field>),
}

#[derive(Debug, Clone)]
pub struct Type {
    pub name: NamespacedName,
    pub kind: TypeKind,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: NamespacedName,
    pub ty: NamespacedName,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub name: NamespacedName,
    pub parts: Vec<Field>,
}

#[derive(Debug, Clone)]
pub struct Operation {
    pub name: NamespacedName,
    pub documentation: Option<String>,
    pub input: Option<NamespacedName>,
    pub output: Option<NamespacedName>,
}

#[derive(Debug, Clone)]
pub struct PortType {
    pub name: NamespacedName,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone)]
pub struct BindingOperation {
    pub name: NamespacedName,
    pub action: String,
    pub style: String,
    pub input: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Binding {
    pub name: NamespacedName,
    pub ty: NamespacedName,
    pub transport: String,
    pub operations: Vec<BindingOperation>,
}

#[derive(Debug, Clone)]
pub struct Port {
    pub name: NamespacedName,
    pub binding: NamespacedName,
    pub location: String,
}

#[derive(Debug, Clone)]
pub struct Service {
    pub name: NamespacedName,
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

impl Namespaces {
    pub fn namespaces(&self) -> &[String] {
        &self.0
    }

    pub fn add_or_get(&mut self, namespace: &str) -> usize {
        if let Some(index) = self.index_of(namespace) {
            index
        } else {
            let index = self.0.len();
            self.0.push(namespace.to_owned());
            index
        }
    }

    fn index_of(&self, namespace: &str) -> Option<usize> {
        self.0.iter().position(|value| value == namespace)
    }
}

impl NamespacedName {
    pub fn new(namespaces: &mut Namespaces, namespace: &str, name: String) -> Self {
        Self {
            namespace_idx: namespaces.add_or_get(namespace),
            name,
        }
    }

    pub fn index(&self) -> usize {
        self.namespace_idx
    }
}
