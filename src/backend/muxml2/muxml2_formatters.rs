#[inline(always)]
pub fn opt_string(c: bool, a: &str) -> &str {
    if c {
        a
    } else {
        ""
    }
}

#[inline]
pub fn write_muxml2_rest(
    buf: &mut impl std::fmt::Write,
    r#type: &str,
    duration: u8,
) -> Result<(), std::fmt::Error> {
    write!(
        buf,
        r#"
<note>
  <rest measure="no"/>
  <duration>{duration}</duration>
  <voice>1</voice>
  <type>{type}</type>
</note>
"#
    )
}

#[inline]
pub fn write_muxml2_note(
    buf: &mut impl std::fmt::Write,
    step: char,
    octave: u8,
    sharp: bool,
    r#type: &str,
    chord: bool,
    dead: bool,
) -> Result<(), std::fmt::Error> {
    let chord_modifier = opt_string(chord, "<chord/>");
    let alter_modifier = opt_string(sharp, "<alter>1</alter>");
    let accidental_modifier = opt_string(sharp, "<accidental>sharp</accidental>");
    let dead_modifier = opt_string(dead, "<notehead>x</notehead>");
    write!(
        buf,
        r#"
<note>
  {chord_modifier}
  <pitch>
    <step>{step}</step>
    {alter_modifier}
    <octave>{octave}</octave>
  </pitch>
  <duration>1</duration>
  <type>{type}</type>
  {accidental_modifier}
  {dead_modifier}
</note>
"#,
    )
}

#[inline]
pub fn write_muxml2_measure(
    buf: &mut impl std::fmt::Write,
    number: usize,
    note_count: usize,
    note_type: usize,
    notes: &str,
) -> Result<(), std::fmt::Error> {
    let first_measure = number == 0;
    let key = opt_string(first_measure, "<key><fifths>0</fifths></key>");
    let clef = opt_string(first_measure, "<clef><sign>G</sign><line>2</line></clef>");

    write!(
        buf,
        r#"
<measure number="{number}">
  <attributes>
    <divisions>2</divisions>
    {key}
    <time>
      <beats>{note_count}</beats>
      <beat-type>{note_type}</beat-type>
    </time>
    {clef}
  </attributes>
  {notes}
</measure>
"#,
    )
}
pub const MUXML_INCOMPLETE_DOC_PRELUDE: &str = r#"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 4.0 Partwise//EN" "http://www.musicxml.org/dtds/partwise.dtd">
<score-partwise version="4.0">
  <identification>
    <encoding>
      <software>guitar_tab</software>
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
