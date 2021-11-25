use std::{fs::File, io::Write, process::Command};

use structopt::StructOpt;
use thiserror::Error;

use suds_wsdl as wsdl;
use suds_codegen as codegen;

#[derive(Debug, Error)]
enum Error {
    #[error("Error parsing WSDL")]
    ParseError(#[from] wsdl::error::Error),

    #[error("Error")]
    IoError(#[from] std::io::Error),
}


#[derive(StructOpt)]
struct Args {
    #[structopt(short, long, default_value="./output.rs")]
    output: String,

    input: String,
}

#[paw::main]
fn main(args: Args) -> Result<(), Error> {
    {
        let tokens = codegen::from_url(args.input)?;
        let mut file = File::create(&args.output)?;
        write!(&mut file, "{}", tokens)?;
    }

    Command::new("rustfmt").arg(args.output).output()?;
    Ok(())
}
