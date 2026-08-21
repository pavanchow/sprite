use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser as ClapParser, Subcommand};
use sprite::Session;

#[derive(ClapParser)]
#[command(name = "sprite", about = "A tiny, embeddable scripting language")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run a .sprite script file
    Run { path: PathBuf },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Run { path }) => run_file(&path),
        None => {
            repl();
            ExitCode::SUCCESS
        }
    }
}

fn run_file(path: &PathBuf) -> ExitCode {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("sprite: cannot read '{}': {}", path.display(), e);
            return ExitCode::FAILURE;
        }
    };
    let mut session = Session::new();
    match session.run(&source) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("sprite: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn repl() {
    println!("sprite 0.1.0 - a tiny embeddable scripting language");
    println!("type an expression or statement, ctrl-d to exit");
    let mut session = Session::new();
    let stdin = io::stdin();
    loop {
        print!("> ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) => {
                println!();
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                match session.run(trimmed) {
                    Ok(sprite::Value::Nil) => {}
                    Ok(v) => println!("{}", v),
                    Err(e) => println!("{}", e),
                }
            }
            Err(e) => {
                eprintln!("sprite: read error: {}", e);
                break;
            }
        }
    }
}
