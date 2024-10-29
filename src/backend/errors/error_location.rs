use yansi::Paint;

#[derive(Clone, Debug)]
pub enum ErrorLocation {
    NoLocation,
    LineOnly(usize),
    LineAndMeasure(usize, usize),
    LineAndCharIdx(usize, usize),
}

impl ErrorLocation {
    pub fn get_line_idx(&self) -> Option<usize> {
        match self {
            ErrorLocation::NoLocation => None,
            ErrorLocation::LineOnly(x) => Some(*x),
            ErrorLocation::LineAndMeasure(x, _) => Some(*x),
            ErrorLocation::LineAndCharIdx(x, _) => Some(*x),
        }
    }
    pub fn write_location_explainer(&self, f: &mut impl std::fmt::Write) {
        match self {
            ErrorLocation::NoLocation => (),
            ErrorLocation::LineOnly(line_idx) => {
                let line_num = line_idx + 1;
                writeln!(f, "{} in line {line_num}:", "Where:".bold(),).unwrap();
            }
            ErrorLocation::LineAndMeasure(line_idx, measure_idx) => {
                let (line_num, measure_num) = (line_idx + 1, measure_idx + 1);
                writeln!(
                    f,
                    "{} Measure {measure_num} in line {line_num}:",
                    "Where:".bold()
                )
                .unwrap();
            }
            ErrorLocation::LineAndCharIdx(line_idx, char_idx) => writeln!(
                f,
                "{} line {} char {}",
                "Where:".bold(),
                line_idx + 1,
                char_idx + 1,
            )
            .unwrap(),
        }
    }
}
