use core::fmt;
use std::fmt::Write;
use std::io::StdoutLock;
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader},
};

use anyhow::Context;
use clap::Parser;
use guitar_tab::backend::errors::backend_error::BackendError;
use guitar_tab::backend::errors::diagnostic::Diagnostic;
use guitar_tab::backend::errors::error_location::ErrorLocation;
use guitar_tab::backend::errors::{extend_error_range, get_digit_cnt};
use guitar_tab::{parser::parser2::parse2, time};
use yansi::{Paint, Painted};

mod cli_args;
use crate::cli_args::Cli;

fn get_lines(input_path: &str) -> anyhow::Result<Vec<String>> {
    if input_path == "-" {
        let mut f = std::io::stdin();
        let lines: Vec<String> = BufReader::new(&mut f).lines().map(|x| x.unwrap()).collect();
        Ok(lines)
    } else {
        let f =
            File::open(input_path).with_context(|| format!("Failed to open file {input_path}"))?;

        let lines: Vec<String> = BufReader::new(&f).lines().map(|x| x.unwrap()).collect();
        // println!("{:?}", lines);
        Ok(lines)
    }
}

enum OutputType {
    File(File),
    Stdout(StdoutLock<'static>),
}
impl std::io::Write for OutputType {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            OutputType::File(x) => x.write(buf),
            OutputType::Stdout(x) => x.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            OutputType::File(x) => x.flush(),
            OutputType::Stdout(x) => x.flush(),
        }
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let input_path = cli.command.input_path();
    let lines: Vec<String> = get_lines(input_path)?;
    let mut diagnostics = vec![];
    let (input_parsing, parsed) = time(|| parse2(lines.iter().map(|x| x.as_str())));
    let parsed = match parsed {
        Ok(mut x) => {
            diagnostics.append(&mut x.diagnostics);
            x
        }
        Err(err) => {
            return handle_error(&err, None, &lines);
        }
    };

    let mut output_fd = if cli.command.output_path() == "-" {
        OutputType::Stdout(std::io::stdout().lock())
    } else {
        let output_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(cli.command.output_path())
            .with_context(|| format!("Failed to open output file {}", cli.command.output_path()))?;
        OutputType::File(output_file)
    };

    let command = &cli.command;
    let backend = command.to_backend_selector();
    let (export, res) = time(|| backend.process(parsed, &mut output_fd));

    match res {
        Ok(mut x) if !x.is_empty() && !cli.quiet => {
            diagnostics.append(&mut x);
            println!(
                "Produced {} diagnostics and no errors",
                diagnostics.len().bold()
            );
            print_diagnostics(diagnostics.iter(), &lines);
        }
        Ok(_empty) => {
            if !cli.quiet {
                print_diagnostics(diagnostics.iter(), &lines)
            }
        }
        Err(x) => handle_error(&x, Some(&diagnostics), &lines)?,
    };
    if !cli.quiet {
        println!("[D]: Performance timings:\nparsing input file: {input_parsing:?}\ncreating file using {command} backend: {export:?}");
    }
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

    println!(
        "Produced {diag_cnt} and {}.",
        "one error".red().bold(),
        diag_cnt = format!("{diag_count} diagnostics").bold(),
    );
    if diag_count != 0 {
        match previous_diags {
            Some(x) => print_diagnostics(x.iter().chain(diagnostics), lines),
            None => print_diagnostics(diagnostics.iter(), lines),
        };
    }

    let mut location_explainer = String::new();
    main_location.write_location_explainer(&mut location_explainer);
    let bold_line_indices = relevant_lines.clone().collect::<Vec<usize>>();
    let max_digit_cnt = get_digit_cnt(*relevant_lines.end());

    for line_idx in extend_error_range(relevant_lines, lines.len()) {
        let zero_pad_cnt = max_digit_cnt - get_digit_cnt(line_idx + 1);
        let mut line_num = String::new();
        for _ in 0..zero_pad_cnt {
            write!(&mut line_num, " ").unwrap();
        }
        write!(&mut line_num, "{}", line_idx + 1).unwrap();

        // a faster .contains() on the range since it is sorted
        let line_num = if bold_line_indices.binary_search(&line_idx).is_ok() {
            line_num.bold()
        } else {
            Painted::new(&line_num)
        };
        writeln!(&mut location_explainer, "{line_num}│ {}", lines[line_idx])?;
        if let ErrorLocation::LineAndCharIdx(e_line_idx, char_idx) = main_location {
            if *e_line_idx != line_idx {
                continue;
            }
            let padding = get_digit_cnt(line_idx) as usize + 2 + *char_idx;
            write_indent(&mut location_explainer, " ", padding);
            writeln!(&mut location_explainer, "{}here", "^".bold()).unwrap();
        }
    }
    let (short, long) = kind.desc();
    println!(
        "\n{first_line}\n{location_explainer}\n{long}",
        first_line = format!("Error: {short}").bold().red()
    );

    Ok(())
}

pub fn print_diagnostics<'a, A: std::iter::Iterator<Item = &'a Diagnostic>>(
    diags: A,
    lines: &[String],
) {
    println!("{}:", "Diagnostics".bold());
    for (
        idx,
        Diagnostic {
            severity,
            kind,
            location,
        },
    ) in diags.enumerate()
    {
        let idx_display = (idx + 1).to_string();
        let mut location_explainer = String::from("\n");
        write_indent(&mut location_explainer, " ", idx_display.len() + 3);
        location.write_location_explainer(&mut location_explainer);
        if let Some(x) = location.get_line_idx() {
            write_indent(&mut location_explainer, " ", idx_display.len() + 3);
            writeln!(&mut location_explainer, "{}│ {}", x + 1, lines[x]).unwrap();
        }
        println!(
            "({idx_display}) {severity}: {kind}{location_explainer}",
            severity = severity.bold()
        );
    }
}

pub fn write_indent(buf: &mut impl fmt::Write, indent: &str, spaces: usize) {
    for _ in 0..spaces {
        write!(buf, "{indent}").unwrap();
    }
}
