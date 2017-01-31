extern crate getopts;

pub mod lexer;
pub mod parser;

use std::io::{self, Write};
use std::env;
use std::process;

fn print_usage(options: &getopts::Options) -> ! {
    print_stderr(format!("{}", options.usage("Usage: inkoc FILE [OPTIONS]")));

    process::exit(1);
}

fn print_stderr(message: String) {
    let mut stderr = io::stderr();

    stderr.write(message.as_bytes()).unwrap();
    stderr.write(b"\n").unwrap();
    stderr.flush().unwrap();
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut options = getopts::Options::new();

    options.optflag("h", "help", "Shows this help message");
    options.optflag("v", "version", "Prints the version number");

    let matches = match options.parse(&args[1..]) {
        Ok(matches) => matches,
        Err(error) => {
            print_stderr(format!("{}", error.to_string()));
            print_usage(&options);
        }
    };

    if matches.opt_present("h") {
        print_usage(&options);
    }

    if matches.opt_present("v") {
        println!("inkoc {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if matches.free.is_empty() {
        print_usage(&options);
    } else {
        let mut parser = parser::Parser::new("'foobar' || 'bar'");
        let ast = parser.parse();

        println!("{:?}", ast);
    }
}
