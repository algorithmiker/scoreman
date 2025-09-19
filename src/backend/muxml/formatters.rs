use crate::backend::muxml::{NoteProperties, Vibrato};
use itoa::Buffer;
use tracing::debug;
// This file uses explicit .write_str() -s, instead of writing a format!()ted string, because I
// benchmarked it and it was faster.
// Maybe there is a nice solution to this - but I've yet to find anything as performant as this one.
#[inline]
pub fn write_muxml2_rest(
    buf: &mut impl std::fmt::Write, r#type: &str, duration: u8,
) -> Result<(), std::fmt::Error> {
    buf.write_str(
        r#"<note>
<rest measure="no"/>
<duration>"#,
    )?;
    let mut dbuf = Buffer::new();
    buf.write_str(dbuf.format(duration))?;
    buf.write_str(
        r#"</duration>
<voice>1</voice>
<type>"#,
    )?;
    buf.write_str(r#type)?;
    buf.write_str("</type>\n</note>\n")?;
    Ok(())
}

#[inline]
pub fn write_muxml2_note(
    buf: &mut impl std::fmt::Write, step: char, octave: u8, sharp: bool, chord: bool, dead: bool,
    properties: Option<&NoteProperties>,
) -> Result<(), std::fmt::Error> {
    buf.write_str("<note>\n")?;
    if chord {
        buf.write_str("<chord/>\n")?
    }
    buf.write_str("<pitch><step>")?;
    buf.write_char(step)?;
    buf.write_str("</step>\n")?;
    if sharp {
        buf.write_str("<alter>1</alter>\n")?
    }
    buf.write_str("<octave>")?;
    let mut octave_buf = itoa::Buffer::new();
    buf.write_str(octave_buf.format(octave))?;
    buf.write_str(
        r#"</octave>
</pitch>
<duration>1</duration>
<type>eighth</type>
"#,
    )?;
    if sharp {
        buf.write_str("<accidental>sharp</accidental>\n")?;
    }
    if dead {
        buf.write_str("<notehead>x</notehead>\n")?;
    }
    match properties {
        None => (),
        Some(NoteProperties { slurs, slide, vibrato }) => {
            debug!(?slurs, "slurs");
            buf.write_str("<notations>\n")?;
            for slur in slurs {
                buf.write_str(r#"<slur type=""#)?;
                buf.write_str(if slur.start { "start" } else { "stop" })?;
                buf.write_str(r#"" number=""#)?;
                buf.write_str(octave_buf.format(slur.number))?;
                buf.write_str("\" />\n")?;
            }
            if let Some(slide) = slide {
                buf.write_str(r#"<slide type=""#)?;
                buf.write_str(if slide.start { "start" } else { "stop" })?;
                buf.write_str(r#"" number=""#)?;
                buf.write_str(octave_buf.format(slide.number))?;
                buf.write_str("\" />\n")?;
            }
            if let Some(vibrato) = vibrato {
                buf.write_str("<ornaments>\n")?;
                buf.write_str("<wavy-line type=\"")?;
                buf.write_str(if matches!(vibrato, Vibrato::Start) { "start" } else { "stop" })?;
                buf.write_str("\" />\n")?;
                buf.write_str("</ornaments>\n")?;
            }
            buf.write_str("</notations>\n")?;
        }
    }
    buf.write_str("</note>\n")?;
    Ok(())
}

#[inline]
pub fn write_muxml2_measure_prelude(
    buf: &mut impl std::fmt::Write, number: usize, note_count: usize, note_type: usize,
) -> Result<(), std::fmt::Error> {
    let first_measure = number == 0;
    buf.write_str(r#"<measure number=""#)?;
    let mut nbuf = Buffer::new();
    buf.write_str(nbuf.format(number))?;
    buf.write_str(
        r#"">
<attributes>
<divisions>2</divisions>
"#,
    )?;
    if first_measure {
        buf.write_str("<key><fifths>0</fifths></key>\n")?
    };
    buf.write_str("<time><beats>")?;
    let mut note_count_buf = Buffer::new();
    buf.write_str(note_count_buf.format(note_count))?;
    buf.write_str("</beats><beat-type>")?;
    let mut note_type_buf = Buffer::new();
    buf.write_str(note_type_buf.format(note_type))?;
    buf.write_str("</beat-type></time>\n")?;
    if first_measure {
        buf.write_str("<clef><sign>G</sign><line>2</line></clef>\n")?
    }
    buf.write_str("</attributes>\n")?;
    Ok(())
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
