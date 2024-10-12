#[inline(always)]
pub fn opt_string(c: bool, a: &str) -> &str {
    if c {
        a
    } else {
        ""
    }
}

#[inline]
pub fn muxml2_rest(r#type: &str, duration: u8) -> String {
    format!(
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
pub fn muxml2_note(
    step: char,
    octave: u8,
    sharp: bool,
    r#type: &str,
    chord: bool,
    dead: bool,
) -> String {
    let chord_modifier = opt_string(chord, "<chord/>");
    let alter_modifier = opt_string(sharp, "<alter>1</alter>");
    let accidental_modifier = opt_string(sharp, "<accidental>sharp</accidental>");
    let dead_modifier = opt_string(dead, "<notehead>x</notehead>");
    format!(
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
pub fn muxml2_measure(number: usize, note_count: usize, note_type: usize, notes: &str) -> String {
    let first_measure = number == 0;
    let key = opt_string(first_measure, "<key><fifths>0</fifths></key>");
    let clef = opt_string(first_measure, "<clef><sign>G</sign><line>2</line></clef>");

    format!(
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

#[inline]
pub fn muxml2_document(measures: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
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
  {measures}
  </part>
</score-partwise>
"#
    )
}
