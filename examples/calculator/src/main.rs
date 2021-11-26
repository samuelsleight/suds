use structopt::StructOpt;

mod calculator {
    use suds_macro::suds;
    suds! {"http://www.dneonline.com/calculator.asmx?WSDL"}
}

pub struct Calculator {
    client: calculator::services::Calculator::CalculatorSoap,
}

impl Calculator {
    pub fn new() -> Self {
        Self {
            client: calculator::services::Calculator::CalculatorSoap::new(),
        }
    }

    pub fn add(&self, a: isize, b: isize) -> isize {
        let result = self.client.Add(calculator::messages::AddSoapIn {
            parameters: calculator::types::Add { intA: a, intB: b },
        });

        result.parameters.AddResult
    }

    pub fn subtract(&self, a: isize, b: isize) -> isize {
        let result = self.client.Subtract(calculator::messages::SubtractSoapIn {
            parameters: calculator::types::Subtract { intA: a, intB: b },
        });

        result.parameters.SubtractResult
    }

    pub fn multiply(&self, a: isize, b: isize) -> isize {
        let result = self.client.Multiply(calculator::messages::MultiplySoapIn {
            parameters: calculator::types::Multiply { intA: a, intB: b },
        });

        result.parameters.MultiplyResult
    }

    pub fn divide(&self, a: isize, b: isize) -> isize {
        let result = self.client.Divide(calculator::messages::DivideSoapIn {
            parameters: calculator::types::Divide { intA: a, intB: b },
        });

        result.parameters.DivideResult
    }
}

#[derive(StructOpt)]
enum Mode {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(StructOpt)]
struct Args {
    #[structopt(subcommand)]
    mode: Mode,

    a: isize,
    b: isize,
}

#[paw::main]
fn main(args: Args) -> Result<(), std::io::Error> {
    let calculator = Calculator::new();

    let result = match args.mode {
        Mode::Add => calculator.add(args.a, args.b),
        Mode::Subtract => calculator.subtract(args.a, args.b),
        Mode::Multiply => calculator.multiply(args.a, args.b),
        Mode::Divide => calculator.divide(args.a, args.b),
    };

    println!("{}", result);
    Ok(())
}
