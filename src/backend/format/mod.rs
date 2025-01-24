//use std::time::Instant;
//
//use clap::ValueEnum;
//
//use super::BackendResult;
//use crate::{
//    backend::Backend,
//    parser::{
//        dump_tracks,
//        parser2::{Parser2, ParserInput},
//        Section,
//    },
//    rlen, time,
//};
//
//pub struct FormatBackend();
//#[derive(ValueEnum, Clone)]
//pub enum FormatDumpOptions {
//    AST,
//    PrettyTracks,
//}
//#[derive(Clone)]
//pub struct FormatBackendSettings {
//    pub dump: Option<FormatDumpOptions>,
//}
//
//impl Backend for FormatBackend {
//    type BackendSettings = FormatBackendSettings;
//
//    fn process<'a, Out: std::io::Write>(
//        parser_input: impl ParserInput<'a>, out: &mut Out, settings: Self::BackendSettings,
//    ) -> BackendResult<'a> {
//        let mut diagnostics = vec![];
//        let parser = Parser2 { track_measures: true, track_sections: true };
//        let (parse_time, parse_result) = match time(|| parser.parse(parser_input)) {
//            (parse_time, Ok(parse_result)) => (parse_time, parse_result),
//            (_, Err(err)) => return BackendResult::new(diagnostics, Some(err), None, None),
//        };
//        match settings.dump {
//            Some(FormatDumpOptions::AST) => println!("{parse_result:?}"),
//            Some(FormatDumpOptions::PrettyTracks) => {
//                println!("{}", dump_tracks(&parse_result.strings, &parse_result.bend_targets));
//            }
//            None => (),
//        }
//        diagnostics.extend(parse_result.diagnostics);
//
//        let gen_start = Instant::now();
//        let mut formatted = String::new();
//        let mut measure_cnt = 0;
//        for section in parse_result.sections {
//            match section {
//                Section::Part { part, .. } => {
//                    let measures_in_part = rlen(&part[0].measures);
//                    for measure_idx in 0..measures_in_part {
//                        formatted += &format!("// SYS: Measure {}\n", measure_cnt + 1);
//                        for (l_idx, line) in part.iter().enumerate() {
//                            formatted.push(line.string_name);
//                            formatted.push('|');
//                            formatted += &parse_result.measures[l_idx][measure_idx]
//                                .print_pretty_string(
//                                    &parse_result.strings[l_idx],
//                                    l_idx as u8,
//                                    &parse_result.bend_targets,
//                                );
//                            formatted.push('|');
//                            formatted.push('\n');
//                        }
//                        formatted.push('\n');
//                        measure_cnt += 1;
//                    }
//                }
//                Section::Comment(x) => {
//                    // SYS-comments were generated during a previous format run, so don't include
//                    // them
//                    if !x.trim_start().starts_with("SYS:") {
//                        formatted += "//";
//                        formatted += &x;
//                        formatted.push('\n');
//                    }
//                }
//            }
//        }
//        let gen_time = gen_start.elapsed();
//
//        if let Err(x) = out.write_all(formatted.as_bytes()) {
//            return BackendResult::new(
//                diagnostics,
//                Some(x.into()),
//                Some(parse_time),
//                Some(gen_time),
//            );
//        }
//
//        BackendResult::new(diagnostics, None, Some(parse_time), Some(gen_time))
//    }
//}
//
