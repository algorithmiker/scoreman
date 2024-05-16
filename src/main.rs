use std::fmt::Write;
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader},
};

use anyhow::Context;
use clap::Parser;
use guitar_tab::{
    backend::errors::{BackendError, Diagnostic},
    parser::parser2::parse2,
    time,
};

mod cli_args;
use crate::cli_args::Cli;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let input_path = cli.command.input_path();
    let f = File::open(input_path).with_context(|| format!("Failed to open file {input_path}"))?;

    let lines: Vec<String> = BufReader::new(&f).lines().map(|x| x.unwrap()).collect();
    let mut diagnostics = vec![];
    let (input_parsing, parsed) = time(|| parse2(lines.iter().map(|x| x.as_str())));
    let parsed = match parsed {
        Ok(mut x) => {
            diagnostics.append(&mut x.0);
            x.1
        }
        Err(err) => {
            return handle_error(&err, None, &lines);
        }
    };

    let mut output_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(cli.command.output_path())
        .with_context(|| format!("Failed to open output file {}", cli.command.output_path()))?;

    let command = &cli.command;
    let backend = command.to_backend_selector();
    let (export, res) = time(|| backend.process(parsed, &mut output_file));

    match res {
        Ok(mut x) if !x.is_empty() => {
            diagnostics.append(&mut x);
            println!(
                "Produced {} diagnostics and no errors.\nDiagnostics:",
                diagnostics.len()
            );
            print_diagnostics(diagnostics.iter(), &lines);
        }
        Ok(_) => (),
        Err(x) => handle_error(&x, Some(&diagnostics), &lines)?,
    };
    println!("[D]: Performance timings:\nparsing input file: {input_parsing:?}\ncreating file using {command} backend: {export:?}");
    Ok(())
}

pub fn handle_error(
    err: &BackendError,
    previous_diags: Option<&[Diagnostic]>,
    lines: &[String],
) -> anyhow::Result<()> {
    let BackendError {
        main_location,
        relevant_lines,
        kind,
        diagnostics,
    } = err;

    let diag_count = diagnostics.len()
        + match previous_diags {
            Some(x) => x.len(),
            _ => 0,
        };

    println!("Produced {diag_count} diagnostics and one error.");
    if diag_count != 0 {
        println!("Diagnostics:\n");
        match previous_diags {
            Some(x) => print_diagnostics(x.iter().chain(diagnostics), lines),
            None => print_diagnostics(diagnostics.iter(), lines),
        };
    }

    let location_explainer = match main_location {
        Some((x, y)) => {
            let (line_num, measure_num) = (x + 1, y + 1);
            let mut tmp = format!("Where: measure {measure_num} in line {line_num}:\n");
            for line_idx in relevant_lines.clone() {
                let line_num = line_idx + 1;
                writeln!(
                    tmp,
                    "{line_num}| {line_content}",
                    line_content = lines[line_idx]
                )?;
            }
            tmp
        }
        None => String::new(),
    };
    let (short, long) = kind.desc();
    println!("\nError: {short}\n{location_explainer}\n{long}");
    Ok(())
}
pub fn print_diagnostics<'a, A: std::iter::Iterator<Item = &'a Diagnostic>>(
    diags: A,
    lines: &[String],
) {
    for (
        idx,
        Diagnostic {
            severity,
            message,
            location,
        },
    ) in diags.enumerate()
    {
        let location_explainer = match location {
            Some((x, y)) => {
                let (line_num, measure_num) = (x + 1, y + 1);
                format!(
                    "measure {measure_num} in line {line_num}:\n    {line_num}| {}",
                    lines[*x]
                )
            }
            None => String::new(),
        };
        println!(
            "  - {}. {severity}: {message}\n    Where: {location_explainer}",
            idx + 1
        );
    }
}
