use crate::backend::muxml::{NoteProperties, Vibrato2};
use itoa::Buffer;
pub enum EnvSpacing {
    /// Newline everywhere, for large tags.
    /// Example:
    /// ```xml
    /// <note>
    ///   ...
    /// <note>
    ///
    /// ```
    /// Writes a newline at the end
    Block,
    /// No newline anywhere, for nested tags.
    /// Example:
    /// ```xml
    /// <pitch><step>5</step></pitch>
    /// ```
    /// No newline, even at the end
    Nested,
    /// Newline at the end but nowhere else. For single-line properties.
    /// ```xml
    /// <foo>bar</foo>
    /// <quux>baz</quux>
    /// ```xml
    Mixed,
}
#[inline(always)]
fn tagged_env<'a, 'b, T: std::fmt::Write, U: FnOnce(&mut T) -> Result<(), std::fmt::Error>>(
    buf: &'b mut T, tag: &'a str, spacing: EnvSpacing,
) -> impl FnOnce(U) -> Result<(), std::fmt::Error> + use<'b, 'a, T, U> {
    move |cb: U| {
        buf.write_char('<')?;
        buf.write_str(tag)?;
        buf.write_char('>')?;
        if let EnvSpacing::Block = spacing {
            buf.write_char('\n')?
        };
        cb(buf)?;
        buf.write_str("</")?;
        buf.write_str(tag)?;
        buf.write_str(">")?;
        match spacing {
            EnvSpacing::Block | EnvSpacing::Mixed => buf.write_char('\n')?,
            _ => (),
        };
        Ok(())
    }
}
#[inline(always)]
fn note_env<T: std::fmt::Write, U: FnOnce(&mut T) -> Result<(), std::fmt::Error>>(
    buf: &mut T,
) -> impl FnOnce(U) -> Result<(), std::fmt::Error> + use<'_, T, U> {
    tagged_env(buf, "note", EnvSpacing::Block)
}

#[inline]
pub fn write_muxml2_rest(
    buf: &mut impl std::fmt::Write, r#type: &str, duration: u8,
) -> Result<(), std::fmt::Error> {
    note_env(buf)(|buf| {
        use EnvSpacing::*;
        buf.write_str("<rest measure=\"no\"/>")?;
        tagged_env(buf, "duration", Mixed)(|buf| buf.write_str(Buffer::new().format(duration)))?;
        tagged_env(buf, "voice", Mixed)(|b| b.write_char('1'))?;
        tagged_env(buf, "type", Mixed)(|b| b.write_str(r#type))
    })
}

#[inline]
pub fn write_muxml2_note(
    buf: &mut impl std::fmt::Write, step: char, octave: u8, sharp: bool, chord: bool, dead: bool,
    properties: Option<&NoteProperties>,
) -> Result<(), std::fmt::Error> {
    note_env(buf)(|buf| {
        use EnvSpacing::*;
        if chord {
            buf.write_str("<chord/>\n")?
        }
        let mut num_buf = Buffer::new();
        tagged_env(buf, "pitch", Mixed)(|buf| {
            tagged_env(buf, "step", Nested)(|buf| buf.write_char(step))?;
            if sharp {
                tagged_env(buf, "alter", Nested)(|buf| buf.write_char('1'))?;
            }
            tagged_env(buf, "octave", Nested)(|buf| buf.write_str(num_buf.format(octave)))
        })?;
        tagged_env(buf, "duration", Mixed)(|buf| buf.write_char('1'))?;
        tagged_env(buf, "type", Mixed)(|buf| buf.write_str("eighth"))?;
        if sharp {
            tagged_env(buf, "accidental", Mixed)(|buf| buf.write_str("sharp"))?;
        }
        if dead {
            tagged_env(buf, "notehead", Mixed)(|buf| buf.write_char('x'))?;
        }
        if let Some(NoteProperties { slurs, slide, vibrato }) = properties {
            tagged_env(buf, "notations", Block)(|buf| {
                for slur in slurs {
                    buf.write_str("<slur type=\"")?;
                    buf.write_str(if slur.start { "start" } else { "stop" })?;
                    buf.write_str("\" number=\"")?;
                    buf.write_str(num_buf.format(slur.number))?;
                    buf.write_str("\" />\n")?;
                }

                if let Some(slide) = slide {
                    buf.write_str(r#"<slide type=""#)?;
                    buf.write_str(if slide.start { "start" } else { "stop" })?;
                    buf.write_str("\" number=\"")?;
                    buf.write_str(num_buf.format(slide.number))?;
                    buf.write_str("\" />\n")?;
                }
                if let Some(v) = vibrato {
                    buf.write_str("<ornaments>\n")?;
                    buf.write_str("<wavy-line type=\"")?;
                    buf.write_str(if matches!(v, Vibrato2::Start) { "start" } else { "stop" })?;
                    buf.write_str("\" />\n")?;
                    buf.write_str("</ornaments>\n")?;
                }
                Ok(())
            })?;
        }
        Ok(())
    })
}
#[inline]
pub fn write_muxml2_measure_prelude(
    buf: &mut impl std::fmt::Write, number: usize, note_count: usize, note_type: usize,
) -> Result<(), std::fmt::Error> {
    let first_measure = number == 0;
    buf.write_str(r#"<measure number=""#)?;
    let mut nbuf = Buffer::new();
    buf.write_str(nbuf.format(number))?;
    buf.write_str("\">\n")?;
    use EnvSpacing::*;
    tagged_env(buf, "attributes", Block)(|buf| {
        tagged_env(buf, "divisions", Nested)(|buf| buf.write_char('2'))?;
        if first_measure {
            buf.write_str(
                "<key><fifths>0</fifths></key>\n<clef><sign>G</sign><line>2</line></clef>",
            )?
        }
        tagged_env(buf, "time", Mixed)(|buf| {
            tagged_env(buf, "beats", Mixed)(|buf| buf.write_str(nbuf.format(note_count)))?;
            tagged_env(buf, "beat-type", Mixed)(|buf| buf.write_str(nbuf.format(note_type)))
        })?;
        Ok(())
    })
}
pub const MUXML_INCOMPLETE_DOC_PRELUDE: &str = r#"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 4.0 Partwise//EN" "http://www.musicxml.org/dtds/partwise.dtd">
<score-partwise version="4.0">
  <identification>
    <encoding>
      <software>scoreman</software>
      <supports element="accidental" type="yes"/>
      <supports element="beam" type="yes"/>
      <supports element="print" attribute="new-page" type="no"/>
      <supports element="print" attribute="new-system" type="no"/>
      <supports element="stem" type="yes"/>
    </encoding>
  </identification>
  <part-list>
    <score-part id="P1">
      <part-name>Guitar1</part-name>
    </score-part>
  </part-list>
  <part id="P1">
"#;
pub const MUXML2_DOCUMENT_END: &str = r#"
</part>
</score-partwise>
"#;
