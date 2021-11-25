use suds_wsdl::types::{self as wsdl, NamespacedName};

#[derive(Debug, Clone)]
pub struct Service {
    pub name: NamespacedName,
    pub ports: Vec<Port>,
}

#[derive(Debug, Clone)]
pub struct Port {
    pub name: NamespacedName,
    pub location: String,
    pub operations: Vec<wsdl::Operation>,
}

#[derive(Default, Debug, Clone)]
pub struct Definition {
    pub services: Vec<Service>,
    pub messages: Vec<wsdl::Message>,
    pub types: Vec<wsdl::Type>,
}
