extern crate proc_macro;

use proc_macro::TokenStream;
use suds_codegen as codegen;
use syn::{parse_macro_input, LitStr};

#[proc_macro]
pub fn suds(input: TokenStream) -> TokenStream {
    let s = parse_macro_input!(input as LitStr);
    codegen::from_url(s.value()).unwrap().into()
}
