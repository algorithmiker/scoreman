use std::{
    fmt::Write,
    fs::{File, OpenOptions},
    io::{BufWriter, Read, StdoutLock},
    sync::Arc,
};

use anyhow::{Context, Ok};
use clap::Parser;
use entrace_core::remote::IETStorage;
use scoreman::{
    backend::errors::{
        backend_error::BackendError, diagnostic::Diagnostic, error_location::ErrorLocation,
        extend_error_range,
    },
    digit_cnt_usize, BufLines, ParseLines,
};
use yansi::{Paint, Painted};
mod cli_args;
use crate::cli_args::Cli;

fn get_file(path: &str) -> anyhow::Result<BufLines> {
    let mut buf = String::new();
    if path == "-" {
        let mut f = std::io::stdin();
        f.read_to_string(&mut buf)?;
    } else {
        let mut file = File::open(path).with_context(|| format!("Failed to open file {path}"))?;
        let size = file.metadata().map(|m| m.len()).unwrap_or(0);
        buf.reserve_exact(size as usize);
        file.read_to_string(&mut buf)?;
    }
    Ok(BufLines::from_string(buf))
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

/// tracing via entrace. don't forget to disable the tracing feature flags too!
#[allow(unreachable_code)]
fn setup_tracing() -> Option<Arc<IETStorage<BufWriter<File>>>> {
    return None;

    use entrace_core::{remote::IETStorage, remote::IETStorageConfig, TreeLayer};
    use std::{fs::OpenOptions, sync::Arc};
    use tracing::level_filters::LevelFilter;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Registry};

    let file =
        OpenOptions::new().write(true).create(true).truncate(true).open("scoreman.iet").unwrap();
    let storage =
        Arc::new(IETStorage::init(IETStorageConfig::non_length_prefixed(BufWriter::new(file))));
    let tree_layer = TreeLayer::from_storage(storage.clone());

    Registry::default().with(LevelFilter::TRACE).with(tree_layer).init();
    Some(storage)
}

fn main() -> anyhow::Result<()> {
    let trace_storage = setup_tracing();
    let cli = Cli::parse();
    let input_path = cli.command.input_path();
    let file_buf = get_file(input_path)?;

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
    let mut result = backend.process(&file_buf, &mut output_fd);

    match &mut result.err {
        Some(x) => handle_error(x, &mut result.diagnostics, &file_buf)?,
        None => {
            if !cli.quiet {
                eprintln!("Produced {} diagnostics and no errors", result.diagnostics.len().bold());
                print_diagnostics(result.diagnostics.iter_mut(), &file_buf);
            }
        }
    }
    if !cli.quiet {
        eprintln!("[D]: Performance timings:");
        match (result.timing_parse, result.timing_gen) {
            (None, None) => eprintln!("Not available"),
            (None, Some(_gen)) => unreachable!(),
            (Some(parse), None) => eprintln!("Parsed file in {parse:?}"),
            (Some(parse), Some(gen)) => {
                eprintln!("Parsed file in {parse:?}\nGenerated output in {gen:?}")
            }
        }
    }
    if let Some(s) = trace_storage {
        s.finish().unwrap();
    }
    if result.err.is_some() {
        std::process::exit(1)
    }
    Ok(())
}

pub fn handle_error(
    err: &mut BackendError, diagnostics: &mut [Diagnostic], lines: &impl ParseLines,
) -> anyhow::Result<()> {
    let BackendError { ref mut main_location, relevant_lines, kind } = err;
    let diag_count = diagnostics.len();

    eprintln!(
        "Produced {} and {}.",
        format!("{diag_count} diagnostics").bold(),
        "one error".red().bold(),
    );
    if diag_count != 0 {
        print_diagnostics(diagnostics.iter_mut(), lines);
    }

    let mut location_explainer = String::new();
    main_location.write_location_explainer(&mut location_explainer);

    let extended_range = extend_error_range(relevant_lines, lines.line_count());
    let max_digit_cnt = digit_cnt_usize(*extended_range.end());
    for line_idx in extended_range {
        let zero_pad_cnt = max_digit_cnt.saturating_sub(digit_cnt_usize(line_idx + 1)) as usize;
        let mut line_num = " ".repeat(zero_pad_cnt);
        write!(&mut line_num, "{}", line_idx + 1)?;

        let line_num = if relevant_lines.contains(&line_idx) {
            line_num.bold()
        } else {
            Painted::new(&line_num)
        };
        writeln!(&mut location_explainer, "{line_num}│ {}", lines.get_line(line_idx))?;
        if let ErrorLocation::LineAndChar(e_line_idx, e_char_idx) = main_location {
            if *e_line_idx as usize != line_idx {
                continue;
            }
            let padding =
                zero_pad_cnt + digit_cnt_usize(line_idx + 1) as usize + 2 + *e_char_idx as usize;
            location_explainer += &" ".repeat(padding);
            writeln!(&mut location_explainer, "{}", "^here".red().bold())?;
        }
    }
    let (short, long) = kind.desc();
    eprintln!(
        "\n{first_line}\n{location_explainer}\n{}",
        long.red(),
        first_line = format!("Error: {short}").bold().red(),
    );

    Ok(())
}

pub fn print_diagnostics<'a, A: Iterator<Item = &'a mut Diagnostic>>(
    diags: A, lines: &impl ParseLines,
) {
    eprintln!("{}:", "Diagnostics".bold());
    for (idx, Diagnostic { severity, kind, location }) in diags.enumerate() {
        let idx_display = (idx + 1).to_string();
        let mut location_explainer = String::from("\n");
        location_explainer += &" ".repeat(idx_display.len() + 3);
        location.write_location_explainer(&mut location_explainer);
        if let Some(x) = location.get_line_idx() {
            location_explainer += &" ".repeat(idx_display.len() + 3);
            writeln!(&mut location_explainer, "{}│ {}", x + 1, lines.get_line(x)).unwrap();
        }
        eprintln!(
            "({idx_display}) {severity}: {kind}{location_explainer}",
            severity = severity.bold()
        );
    }
}
