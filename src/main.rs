use core::fmt;
use std::{
    fmt::Write,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, StdoutLock},
};

use anyhow::Context;
use clap::Parser;
use guitar_tab::backend::errors::{
    backend_error::BackendError, diagnostic::Diagnostic, error_location::ErrorLocation,
    extend_error_range, get_digit_cnt,
};
use yansi::{Paint, Painted};

mod cli_args;
use crate::cli_args::Cli;

// TODO: error reporting without slurping up the whole file
// The parser already works on a streaming basis, it's only the printing of errors which requires
// this
//
// Not very high priority, because it is not that slow.
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

/// TODO: fix the GUI and merge this into this workspace
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let input_path = cli.command.input_path();
    let lines: Vec<String> = get_lines(input_path)?;

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
    let mut result = backend.process(lines.iter().map(|x| x.as_str()), &mut output_fd);
    match result.err {
        Some(mut x) => handle_error(&mut x, &mut result.diagnostics, &lines)?,
        None => {
            if !result.diagnostics.is_empty() && !cli.quiet {
                println!(
                    "Produced {} diagnostics and no errors",
                    result.diagnostics.len().bold()
                );
                print_diagnostics(result.diagnostics.iter_mut(), &lines);
            }
        }
    }
    if !cli.quiet {
        println!("[D]: Performance timings:");
        match (result.timing_parse, result.timing_gen) {
            (None, None) => println!("Not available"),
            (None, Some(_gen)) => unreachable!(),
            (Some(parse), None) => println!("Parsed file in {parse:?}"),
            (Some(parse), Some(gen)) => {
                println!("Parsed file in {parse:?}\nGenerated output in {gen:?}")
            }
        }
    }
    Ok(())
}

pub fn handle_error(
    err: &mut BackendError,
    diagnostics: &mut [Diagnostic],
    lines: &[String],
) -> anyhow::Result<()> {
    let BackendError {
        ref mut main_location,
        relevant_lines,
        kind,
    } = err;
    let diag_count = diagnostics.len();

    println!(
        "Produced {} and {}.",
        format!("{diag_count} diagnostics").bold(),
        "one error".red().bold(),
    );
    if diag_count != 0 {
        print_diagnostics(diagnostics.iter_mut(), lines);
    }

    let mut location_explainer = String::new();
    main_location.write_location_explainer(&mut location_explainer, lines);

    // TODO: make this a range contains
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
        if let ErrorLocation::SourceOffset(src_offset) = main_location {
            let (e_line_idx, e_char_idx) = src_offset.get_line_char(lines);
            if e_line_idx != line_idx {
                continue;
            }
            let padding = get_digit_cnt(line_idx) as usize + 2 + e_char_idx;
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

pub fn print_diagnostics<'a, A: std::iter::Iterator<Item = &'a mut Diagnostic>>(
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
        location.write_location_explainer(&mut location_explainer, lines);
        if let Some(x) = location.get_line_idx(lines) {
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
