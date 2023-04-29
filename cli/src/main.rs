#![allow(unused)]

use clap::Parser;
use std::io::BufRead;
use std::path::PathBuf;

use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};

/// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
struct Cli {
    /// The path to the file to read
    path: Option<std::path::PathBuf>,
}

fn repl() {
    let mut rl = DefaultEditor::new().unwrap();
    let mut lines = String::new();

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                lines.push_str(line.as_str());
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                let expr = lines.as_str();
                if let Err(e) = rlox::run(expr) {
                    println!("ERROR: {:#?}", e);
                }
                lines.clear();
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        };
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
