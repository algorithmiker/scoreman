use yansi::Paint;

#[derive(Clone, Debug)]
pub enum ErrorLocation {
    NoLocation,
    LineOnly(usize),
    LineAndMeasure(usize, usize),
    SourceOffset(SourceOffset),
    LineAndChar(u32, u32),
}
#[derive(Clone, Debug)]
pub struct SourceOffset {
    pub offset: usize,
    pub resolved: Option<(usize, usize)>,
}
impl SourceOffset {
    fn resolve(&mut self, lines: &[String]) {
        let mut line_start = 0;
        let mut line_idx = 0;
        while line_idx < lines.len() && line_start + lines[line_idx].len() + 1 < self.offset {
            line_idx += 1;
            line_start += lines[line_idx].len() + 1;
        }
        self.resolved = Some((line_idx, self.offset - line_start));
        //println!("resolved  {self:?}");
    }
    pub fn new(offset: usize) -> Self {
        Self { offset, resolved: None }
    }
    pub fn get_line_char(&mut self, lines: &[String]) -> (usize, usize) {
        if let Some(x) = self.resolved {
            x
        } else {
            self.resolve(lines);
            self.resolved.unwrap()
        }
    }
}
impl ErrorLocation {
    /// this may not be cheap!
    pub fn get_line_idx(&mut self, lines: &[String]) -> Option<usize> {
        match self {
            ErrorLocation::NoLocation => None,
            ErrorLocation::LineOnly(x) => Some(*x),
            ErrorLocation::LineAndMeasure(x, _) => Some(*x),
            ErrorLocation::SourceOffset(offset) => Some(offset.get_line_char(lines).0),
            ErrorLocation::LineAndChar(l, o) => Some(*l as usize),
        }
    }
    pub fn write_location_explainer(&mut self, f: &mut impl std::fmt::Write, lines: &[String]) {
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
            ErrorLocation::SourceOffset(src_offset) => {
                let (line, char) = src_offset.get_line_char(lines);
                writeln!(f, "{} line {} char {}", "Where:".bold(), line + 1, char + 1).unwrap()
            }
            ErrorLocation::LineAndChar(line, char) => {
                writeln!(f, "{} line {} char {}", "Where:".bold(), line, char).unwrap()
            }
        }
    }
}
