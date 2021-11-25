use proc_macro2::TokenStream;
use suds_wsdl::{
    self as wsdl, error,
    types::{Definition, Namespaces},
};

use codegen::Codegen;

mod codegen;
mod preprocessor;
mod types;

pub fn from_url<S: AsRef<str>>(url: S) -> Result<TokenStream, error::Error> {
    let (definition, namespaces) = wsdl::parse(url)?;
    from_definition(&definition, &namespaces)
}

pub fn from_definition(
    definition: &Definition,
    namespaces: &Namespaces,
) -> Result<TokenStream, error::Error> {
    let definition = preprocessor::preprocess(definition);
    Ok(definition.codegen(namespaces))
}
