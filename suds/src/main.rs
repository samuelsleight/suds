use std::{fs::File, io::Write};

use structopt::StructOpt;
use thiserror::Error;

use suds_codegen as codegen;
use suds_wsdl as wsdl;

#[derive(Debug, Error)]
enum Error {
    #[error("Error parsing WSDL")]
    ParseError(#[from] wsdl::error::Error),

    #[error("Error handling output")]
    SynError(#[from] syn::Error),

    #[error("Error")]
    IoError(#[from] std::io::Error),
}

#[derive(StructOpt)]
struct Args {
    #[structopt(short, long, default_value = "./output.rs")]
    output: String,

    input: String,
}

#[paw::main]
fn main(args: Args) -> Result<(), Error> {
    {
        let tokens = codegen::from_url(args.input)?;
        let ast: syn::File = syn::parse2(tokens)?;

        let mut file = File::create(&args.output)?;
        write!(&mut file, "{}", prettyplease::unparse(&ast))?;
    }

    Ok(())
}
