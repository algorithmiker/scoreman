use super::{Backend, BackendError, Diagnostic};
use crate::parser::Section;

pub struct FormatBackend();

impl Backend for FormatBackend {
    type BackendSettings = ();

    fn process<Out: std::io::Write>(
        score: crate::parser::Score,
        out: &mut Out,
        _settings: Self::BackendSettings,
    ) -> Result<Vec<Diagnostic>, BackendError> {
        let diagnostics = vec![];
        let mut formatted = String::new();
        let mut measure_cnt = 0;
        for section in score.0 {
            match section {
                Section::Part { part, .. } => {
                    let measures_in_part = part[0].staffs.len();
                    for measure_idx in 0..measures_in_part {
                        formatted += &format!("// SYS: Measure {}\n", measure_cnt + 1);
                        for line in &part {
                            formatted.push(line.string_name);
                            formatted.push('|');
                            formatted += &line.staffs[measure_idx].print_pretty_string();
                            formatted.push('|');
                            formatted.push('\n');
                        }
                        formatted.push('\n');
                        measure_cnt += 1;
                    }
                }
                Section::Comment(x) => {
                    // SYS-comments were generated during a previous format run, so don't include
                    // them
                    if !x.trim_start().starts_with("SYS:") {
                        formatted += "//";
                        formatted += &x;
                        formatted.push('\n');
                    }
                }
            }
        }

        if let Err(x) = out.write_all(formatted.as_bytes()) {
            return Err(BackendError::from_io_error(x, diagnostics));
        }

        Ok(diagnostics)
    }
}
