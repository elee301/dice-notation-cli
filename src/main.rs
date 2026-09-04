mod dice;
mod rng;

use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::process::ExitCode;

fn print_usage() {
    eprintln!("usage: diceroll [EXPRESSION]");
    eprintln!("       diceroll -f FILE");
    eprintln!("       diceroll -                 (read expressions from stdin)");
    eprintln!("       echo '2d6+3' | diceroll     (no args also reads stdin)");
    eprintln!();
    eprintln!("Each line is one dice expression, e.g. 3d6+2, d20-1, 4d4+2d6-3.");
    eprintln!("Append '!' after the side count for exploding dice, e.g. 3d6!.");
    eprintln!("Append 'r' and a threshold to reroll low dice once, e.g. 4d6r2.");
    eprintln!("Blank lines and lines starting with '#' are skipped.");
}

fn roll_lines<I: Iterator<Item = String>>(lines: I) -> ExitCode {
    let mut generator = rng::Rng::new();
    let mut had_error = false;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        match dice::parse(trimmed) {
            Ok(expr) => {
                let result = expr.roll(&mut generator);
                println!("{}: {}", trimmed, result);
            }
            Err(e) => {
                eprintln!("{}: error: {}", trimmed, e);
                had_error = true;
            }
        }
    }

    if had_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn read_stdin_lines() -> Vec<String> {
    io::stdin().lock().lines().filter_map(Result::ok).collect()
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        return roll_lines(read_stdin_lines().into_iter());
    }

    if args[0] == "-h" || args[0] == "--help" {
        print_usage();
        return ExitCode::SUCCESS;
    }

    if args[0] == "-" {
        return roll_lines(read_stdin_lines().into_iter());
    }

    if args[0] == "-f" || args[0] == "--file" {
        let Some(path) = args.get(1) else {
            eprintln!("error: {} requires a file path", args[0]);
            return ExitCode::FAILURE;
        };
        let contents = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error reading '{}': {}", path, e);
                return ExitCode::FAILURE;
            }
        };
        let lines: Vec<String> = contents.lines().map(String::from).collect();
        return roll_lines(lines.into_iter());
    }

    // Anything else is treated as a single expression, joined back together
    // so "2d6 + 3" typed as separate shell words still works.
    let expression = args.join(" ");
    roll_lines(std::iter::once(expression))
}
