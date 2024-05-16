use crate::parser::{
    Measure, Score,
    TabElement::{self, Fret},
};

use super::{muxml2::fretboard::get_fretboard_note, Backend, BackendError, Diagnostic};

pub struct MuxmlBackend();
impl Backend for MuxmlBackend {
    type BackendSettings = ();

    fn process<Out: std::io::Write>(
        score: Score,
        out: &mut Out,
        _settings: Self::BackendSettings,
    ) -> Result<Vec<Diagnostic>, BackendError> {
        let mut diagnostics=vec! [
            Diagnostic::warn(None,"The MUXML1 backend is significantly worse than the MUXML2 backend. If you don't have any reason not to, use the MUXML2 backend".into()),
        ];
        let raw_tracks = score.gen_raw_tracks()?;
        let (xml_out, mut xml_diagnostics) = raw_tracks_to_xml(raw_tracks)?;
        diagnostics.append(&mut xml_diagnostics);
        diagnostics.push(Diagnostic::info(
            None,
            "Generated an Uncompressed MusicXML (.musicxml) file.".into(),
        ));
        diagnostics.push(Diagnostic::info(
            None,
            r#"The 6 strings of the guitar are labelled as separate instruments. To fix that,
     1. import the generated file into MuseScore
     2. select all tracks, do Tools->Implode
     3. delete all other tracks except the first."#
                .into(),
        ));
        if let Err(x) = out.write_all(xml_out.as_bytes()) {
            return Err(BackendError::from_io_error(x, diagnostics));
        }
        Ok(diagnostics)
    }
}

fn raw_tracks_to_xml<'a>(
    raw_tracks: ([char; 6], [Vec<Measure>; 6]),
) -> Result<(String, Vec<Diagnostic>), BackendError<'a>> {
    let mut parts_xml = String::new();
    let diagnostics = vec![];
    for i in 0..6 {
        let part = &raw_tracks.1[i];
        let mut measures_xml = String::new();
        for (measure_idx, measure) in part.iter().enumerate() {
            let m_par_line = measure.parent_line.unwrap();
            let m_idx_on_par_line = measure.index_on_parent_line.unwrap();
            let loc = (m_par_line, m_idx_on_par_line);

            let mut notes_xml = String::new();
            for note in &measure.content {
                match note {
                    Fret(fret) => {
                        let x = get_fretboard_note(raw_tracks.0[i], *fret, loc, &diagnostics)?;
                        notes_xml.push_str(&x.into_muxml("eighth", false));
                    }
                    TabElement::DeadNote => {
                        let mut x = get_fretboard_note(raw_tracks.0[i], 0, loc, &diagnostics)?;
                        notes_xml.push_str(&x.into_muxml("eighth", false));
                        x.dead = true;
                        notes_xml.push_str(&x.into_muxml("eighth", false));
                    }
                    TabElement::Rest => notes_xml.push_str(&muxml_rest("eighth", 1)),
                }
            }
            //println!("[D]: finished {measure:?}");
            measures_xml.push_str(&muxml_measure(
                measure_idx as u32,
                measure.content.len(),
                &notes_xml,
            ));
        }
        // musescore numbers p1 to p6, so we do that too for nicer diffs
        parts_xml.push_str(&muxml_part(i as u32 + 1, &measures_xml));
    }

    Ok((muxml_document(&parts_xml), diagnostics))
}

fn muxml_rest(r#type: &str, duration: u8) -> String {
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

fn muxml_measure(number: u32, note_count: usize, notes: &str) -> String {
    let (key, clef) = if number == 0 {
        (
            r#"<key>
  <fifths>0</fifths>
</key>"#,
            r#"<clef>
  <sign>G</sign>
  <line>2</line>
</clef>
"#,
        )
    } else {
        ("", "")
    };

    format!(
        r#"
<measure number="{number}">
  <attributes>
    <divisions>2</divisions>
    {key}
    <time>
      <beats>{note_count}</beats>
      <beat-type>8</beat-type>
    </time>
    {clef} 
  </attributes>
  {notes}
</measure>
"#,
    )
}

fn muxml_part(number: u32, measures: &str) -> String {
    format!(
        r#"
<part id="P{number}">
  {measures}
</part>
    "#
    )
}

fn muxml_document(parts: &str) -> String {
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
    <score-part id="P2">
      <part-name>Guitar2</part-name>
    </score-part>
    <score-part id="P3">
      <part-name>Guitar3</part-name>
    </score-part>
    <score-part id="P4">
      <part-name>Guitar4</part-name>
    </score-part>
    <score-part id="P5">
      <part-name>Guitar5</part-name>
    </score-part>
    <score-part id="P6">
      <part-name>Guitar6</part-name>
    </score-part>
  </part-list>
  {parts}
</score-partwise>
"#
    )
}
