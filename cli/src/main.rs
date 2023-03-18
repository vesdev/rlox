#![allow(unused)]

use clap::Parser;
use std::io::BufRead;
use std::path::PathBuf;
/// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
struct Cli {
    /// The path to the file to read
    path: Option<std::path::PathBuf>,
}

fn repl() {
    let stdin = std::io::stdin();
    loop {
        let mut line = String::new();
        stdin.lock().read_line(&mut line).unwrap();
        let expr = line.as_str();
        println!("> {}", expr);
        if let Err(e) = rlox::run(expr) {
            println!("ERROR: {:#?}", e);
        }
    }
}

fn main() {
    let args = Cli::parse();

    if let Some(path) = args.path {
        if let Err(e) = rlox::run_file(path) {
            println!("ERROR: {:#?}", e);
        }
    } else {
        repl();
    }
}
