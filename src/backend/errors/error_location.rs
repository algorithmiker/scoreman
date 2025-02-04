use yansi::Paint;

#[derive(Clone, Debug, PartialEq)]
pub enum ErrorLocation {
    NoLocation,
    LineOnly(usize),
    LineAndMeasure(usize, usize),
    LineAndChar(u32, u32),
}
#[derive(Clone, Debug)]
pub struct SourceOffset {
    pub offset: usize,
    pub resolved: Option<(usize, usize)>,
}

impl ErrorLocation {
    pub fn get_line_idx(&self) -> Option<usize> {
        match self {
            ErrorLocation::NoLocation => None,
            ErrorLocation::LineOnly(x) => Some(*x),
            ErrorLocation::LineAndMeasure(x, _) => Some(*x),
            ErrorLocation::LineAndChar(l, _) => Some(*l as usize),
        }
    }
    pub fn get_char_idx(&self) -> Option<usize> {
        match self {
            ErrorLocation::NoLocation
            | ErrorLocation::LineOnly(..)
            | ErrorLocation::LineAndMeasure(..) => None,
            ErrorLocation::LineAndChar(_, c) => Some((*c) as usize),
        }
    }
    pub fn write_location_explainer(&self, f: &mut impl std::fmt::Write) {
        match self {
            ErrorLocation::NoLocation => (),
            ErrorLocation::LineOnly(line_idx) => {
                let line_num = *line_idx + 1;
                writeln!(f, "{} in line {line_num}:", "Where:".bold(),).unwrap();
            }
            ErrorLocation::LineAndMeasure(line_idx, measure_idx) => {
                let (line_num, measure_num) = (*line_idx + 1, *measure_idx + 1);
                writeln!(f, "{} Measure {measure_num} in line {line_num}:", "Where:".bold())
                    .unwrap();
            }
            ErrorLocation::LineAndChar(line, char) => {
                writeln!(f, "{} line {} char {}", "Where:".bold(), *line + 1, *char + 1).unwrap()
            }
        }
    }
}
