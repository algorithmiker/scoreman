/// TODO: extract common formatters between muxml and muxml2
use crate::backend::muxml2::muxml2_formatters::write_muxml2_note;
use crate::{
    backend::errors::diagnostic_kind::DiagnosticKind,
    parser::{
        parser2::{Parse2Result, Parser2, ParserInput},
        TabElement::{self, Fret},
    },
    rlen, time,
};

use super::muxml2::ToMuxml;
use super::{
    muxml2::fretboard::get_fretboard_note2, Backend, BackendError, BackendResult, Diagnostic,
};

pub struct MuxmlBackend();
impl Backend for MuxmlBackend {
    type BackendSettings = ();

    fn process<'a, Out: std::io::Write>(
        input: impl ParserInput<'a>,
        out: &mut Out,
        _settings: Self::BackendSettings,
    ) -> BackendResult<'a> {
        let mut diagnostics = vec![Diagnostic::warn(NoLocation, DiagnosticKind::Muxml1IsBad)];
        let parser = Parser2 {
            track_measures: true,
            track_sections: false,
        };

        let (parse_time, parse_result) = match time(|| parser.parse(input)) {
            (parse_time, Ok(parse_result)) => (parse_time, parse_result),
            (_, Err(err)) => return BackendResult::new(diagnostics, Some(err), None, None),
        };
        use super::errors::error_location::ErrorLocation::*;
        let (gen_time, xml_out, mut xml_diagnostics) = match time(|| gen_muxml1(parse_result)) {
            (gen_time, Ok((xml_out, xml_diagnostics))) => (gen_time, xml_out, xml_diagnostics),
            (gen_time, Err(x)) => {
                return BackendResult::new(diagnostics, Some(x), Some(parse_time), Some(gen_time))
            }
        };
        diagnostics.append(&mut xml_diagnostics);
        diagnostics.push(Diagnostic::info(
            NoLocation,
            DiagnosticKind::Muxml1SeperateTracks,
        ));
        if let Err(x) = out.write_all(xml_out.as_bytes()) {
            return BackendResult::new(
                diagnostics,
                Some(x.into()),
                Some(parse_time),
                Some(gen_time),
            );
        }
        BackendResult::new(diagnostics, None, Some(parse_time), Some(gen_time))
    }
}

fn gen_muxml1<'a>(
    parse_result: Parse2Result,
) -> Result<(String, Vec<Diagnostic>), BackendError<'a>> {
    let mut parts_xml = String::new();
    let diagnostics = vec![];
    let mut slur_cnt = 0;
    for i in 0..6 {
        let mut measures_xml = String::new();
        for (measure_idx, measure) in parse_result.measures[i].iter().enumerate() {
            let mut notes_xml = String::new();
            for tick_idx in measure.content.clone() {
                let raw_tick = &parse_result.strings[i][tick_idx];
                raw_tick.element.write_muxml(
                    &mut notes_xml,
                    parse_result.string_names[i],
                    false,
                    &mut slur_cnt,
                    // TODO: pull this out of muxml2 and make configurable
                    super::muxml2::settings::Muxml2BendMode::EmulateBends,
                    &parse_result.bend_targets.get(&(i as u8, tick_idx as u32)),
                )?;
            }
            //println!("[D]: finished {measure:?}");
            measures_xml.push_str(&muxml_measure(
                measure_idx as u32,
                rlen(&measure.content),
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
