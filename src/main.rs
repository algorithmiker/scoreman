use std::{
    fs::{File, OpenOptions},
    io::BufReader,
};

use anyhow::Context;
use clap::Parser;
use guitar_tab::{parser::parser2::parse2, time};

mod cli_args;
use crate::cli_args::Cli;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let (read_input, input) = time::<anyhow::Result<BufReader<File>>, _>(|| {
        let input_path = cli.command.input_path();
        let f =
            File::open(input_path).with_context(|| format!("Failed to open file {input_path}"))?;
        Ok(BufReader::new(f))
    });
    let input = input?;

    let (input_parsing, parsed) = time(|| parse2(input));
    let parsed = parsed?;

    let mut output_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(cli.command.output_path())
        .with_context(|| format!("Failed to open output file {}", cli.command.output_path()))?;

    let command = &cli.command;
    let backend = command.to_backend_selector();
    let (export, res) = time(|| backend.process(parsed, &mut output_file));
    res?;
    println!("[D]: Performance timings: \nreading input file: {read_input:?}\nparsing input file: {input_parsing:?}\ncreating file using {command} backend: {export:?}");
    Ok(())
}
