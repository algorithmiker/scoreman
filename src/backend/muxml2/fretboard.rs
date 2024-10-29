use std::{cell::RefCell, collections::HashMap};

use crate::backend::errors::{backend_error::BackendError, diagnostic::Diagnostic};

use super::MuxmlNote;
use anyhow::bail;

fn note(note: char, octave: u8) -> MuxmlNote {
    MuxmlNote {
        step: note,
        octave,
        sharp: false,
        dead: false,
    }
}
fn note_sharp(note: char, octave: u8) -> MuxmlNote {
    MuxmlNote {
        step: note,
        octave,
        sharp: true,
        dead: false,
    }
}

impl MuxmlNote {
    pub fn next_note(&self) -> anyhow::Result<MuxmlNote> {
        let new = self.clone();
        new.next_note_consuming()
    }
    pub fn next_note_consuming(mut self) -> anyhow::Result<MuxmlNote> {
        let (next_step, next_sharp, octave_diff) = match (self.step, self.sharp) {
            ('C', false) => ('C', true, 0),
            ('C', true) => ('D', false, 0),
            ('D', false) => ('D', true, 0),
            ('D', true) => ('E', false, 0),
            ('E', false) => ('F', false, 0),
            ('E', true) => unreachable!("E# in mscore backend"),
            ('F', false) => ('F', true, 0),
            ('F', true) => ('G', false, 0),
            ('G', false) => ('G', true, 0),
            ('G', true) => ('A', false, 0),
            ('A', false) => ('A', true, 0),
            ('A', true) => ('B', false, 0),
            ('B', false) => ('C', false, 1),
            ('B', true) => unreachable!("B# in mscore backend"),
            (note, sharp) => {
                bail!(
                    "Don't know what note comes after {note}{}",
                    if sharp { "#" } else { "" }
                )
            }
        };
        self.step = next_step;
        self.sharp = next_sharp;
        self.octave += octave_diff;

        Ok(self)
    }
}

pub fn gen_note_cache() {
    NOTE_CACHE.with_borrow(|cache: &HashMap<(char, u16), MuxmlNote>| {
        for ((string, _), note) in cache {
            let mut note = note.clone();
            for fret in 0..13 {
                let MuxmlNote {
                    step,
                    octave,
                    sharp,
                    ..
                } = note;
                let note_builder = if sharp { "note_sharp" } else { "note" };
                println!("h.insert(('{string}', {fret}), {note_builder}('{step}', {octave}));");
                note = note.next_note_consuming().unwrap();
            }
        }
    });
}
/// location is (line_idx,measure_idx)
pub fn get_fretboard_note(
    string: char,
    fret: u16,
    location: (usize, usize),
    diagnostics: &[Diagnostic],
) -> Result<MuxmlNote, BackendError<'static>> {
    NOTE_CACHE.with_borrow_mut(|v| {
        if let Some(x) = v.get(&(string, fret)) {
            Ok(x.clone())
        } else {
            let base_note = &v[&(string, 0)];
            let mut idx = 0;
            let mut current_note = base_note.clone();
            while idx < fret {
                current_note = match current_note.next_note_consuming() {
                    // todo add x to diagnostics
                    Err(_) => {
                        return Err(BackendError::no_such_fret(
                            location.0,
                            location.1,
                            string,
                            fret,
                            diagnostics.to_vec(),
                        ))
                    }
                    Ok(x) => x,
                };

                idx += 1;
                v.insert((string, idx), current_note.clone());
            }

            Ok(current_note)
        }
    })
}

thread_local! {
  static NOTE_CACHE: RefCell<HashMap<(char, u16), MuxmlNote>> =  {
    let mut h = HashMap::with_capacity(13*7);

    h.insert(('D', 0), note('D', 4));
    h.insert(('D', 1), note_sharp('D', 4));
    h.insert(('D', 2), note('E', 4));
    h.insert(('D', 3), note('F', 4));
    h.insert(('D', 4), note_sharp('F', 4));
    h.insert(('D', 5), note('G', 4));
    h.insert(('D', 6), note_sharp('G', 4));
    h.insert(('D', 7), note('A', 4));
    h.insert(('D', 8), note_sharp('A', 4));
    h.insert(('D', 9), note('B', 4));
    h.insert(('D', 10), note('C', 5));
    h.insert(('D', 11), note_sharp('C', 5));
    h.insert(('D', 12), note('D', 5));

    h.insert(('E', 0), note('E', 3));
    h.insert(('E', 1), note('F', 3));
    h.insert(('E', 2), note_sharp('F', 3));
    h.insert(('E', 3), note('G', 3));
    h.insert(('E', 4), note_sharp('G', 3));
    h.insert(('E', 5), note('A', 3));
    h.insert(('E', 6), note_sharp('A', 3));
    h.insert(('E', 7), note('B', 3));
    h.insert(('E', 8), note('C', 4));
    h.insert(('E', 9), note_sharp('C', 4));
    h.insert(('E', 10), note('D', 4));
    h.insert(('E', 11), note_sharp('D', 4));
    h.insert(('E', 12), note('E', 4));

    h.insert(('A', 0), note('A', 3));
    h.insert(('A', 1), note_sharp('A', 3));
    h.insert(('A', 2), note('B', 3));
    h.insert(('A', 3), note('C', 4));
    h.insert(('A', 4), note_sharp('C', 4));
    h.insert(('A', 5), note('D', 4));
    h.insert(('A', 6), note_sharp('D', 4));
    h.insert(('A', 7), note('E', 4));
    h.insert(('A', 8), note('F', 4));
    h.insert(('A', 9), note_sharp('F', 4));
    h.insert(('A', 10), note('G', 4));
    h.insert(('A', 11), note_sharp('G', 4));
    h.insert(('A', 12), note('A', 4));

    h.insert(('d', 0), note('D', 5));
    h.insert(('d', 1), note_sharp('D', 5));
    h.insert(('d', 2), note('E', 5));
    h.insert(('d', 3), note('F', 5));
    h.insert(('d', 4), note_sharp('F', 5));
    h.insert(('d', 5), note('G', 5));
    h.insert(('d', 6), note_sharp('G', 5));
    h.insert(('d', 7), note('A', 5));
    h.insert(('d', 8), note_sharp('A', 5));
    h.insert(('d', 9), note('B', 5));
    h.insert(('d', 10), note('C', 6));
    h.insert(('d', 11), note_sharp('C', 6));
    h.insert(('d', 12), note('D', 6));

    h.insert(('G', 0), note('G', 4));
    h.insert(('G', 1), note_sharp('G', 4));
    h.insert(('G', 2), note('A', 4));
    h.insert(('G', 3), note_sharp('A', 4));
    h.insert(('G', 4), note('B', 4));
    h.insert(('G', 5), note('C', 5));
    h.insert(('G', 6), note_sharp('C', 5));
    h.insert(('G', 7), note('D', 5));
    h.insert(('G', 8), note_sharp('D', 5));
    h.insert(('G', 9), note('E', 5));
    h.insert(('G', 10), note('F', 5));
    h.insert(('G', 11), note_sharp('F', 5));
    h.insert(('G', 12), note('G', 5));

    h.insert(('B', 0), note('B', 4));
    h.insert(('B', 1), note('C', 5));
    h.insert(('B', 2), note_sharp('C', 5));
    h.insert(('B', 3), note('D', 5));
    h.insert(('B', 4), note_sharp('D', 5));
    h.insert(('B', 5), note('E', 5));
    h.insert(('B', 6), note('F', 5));
    h.insert(('B', 7), note_sharp('F', 5));
    h.insert(('B', 8), note('G', 5));
    h.insert(('B', 9), note_sharp('G', 5));
    h.insert(('B', 10), note('A', 5));
    h.insert(('B', 11), note_sharp('A', 5));
    h.insert(('B', 12), note('B', 5));

    h.insert(('e', 0), note('E', 5));
    h.insert(('e', 1), note('F', 5));
    h.insert(('e', 2), note_sharp('F', 5));
    h.insert(('e', 3), note('G', 5));
    h.insert(('e', 4), note_sharp('G', 5));
    h.insert(('e', 5), note('A', 5));
    h.insert(('e', 6), note_sharp('A', 5));
    h.insert(('e', 7), note('B', 5));
    h.insert(('e', 8), note('C', 6));
    h.insert(('e', 9), note_sharp('C', 6));
    h.insert(('e', 10), note('D', 6));
    h.insert(('e', 11), note_sharp('D', 6));
    h.insert(('e', 12), note('E', 6));

    RefCell::new(h)
  };
}
//#[allow(clippy::declare_interior_mutable_const)]
//pub const CLASSICAL_FRETBOARD: Lazy<HashMap<char, [MuxmlNote; 16]>> = Lazy::new(|| {
//    let mut h = HashMap::new();
//    h.insert(
//        'e',
//        [
//            note('E', 5),
//            note("F", 5),
//            note_sharp("F", 5),
//            note("G", 5),
//            note_sharp("G", 5),
//            note("A", 5),
//            note_sharp("A", 5),
//            note("B", 5),
//            note("C", 6),
//            note_sharp("C", 6),
//            note("D", 6),
//            note_sharp("D", 6),
//            note("E", 6),
//            note("F", 6),
//            note_sharp("F", 6),
//            note("G", 6),
//        ],
//    );
//    h.insert(
//        'd',
//        [
//            note("D", 5),
//            note_sharp("D", 5),
//            note("E", 5),
//            note("F", 5),
//            note_sharp("F", 5),
//            note("G", 5),
//            note_sharp("G", 5),
//            note("A", 5),
//            note_sharp("A", 5),
//            note("B", 5),
//            note("C", 6),
//            note_sharp("C", 6),
//            note("D", 6),
//            note_sharp("D", 6),
//            note("E", 6),
//            note("F", 6),
//        ],
//    );
//    h.insert(
//        'B',
//        [
//            note("B", 4),
//            note("C", 5),
//            note_sharp("C", 5),
//            note("D", 5),
//            note_sharp("D", 5),
//            note("E", 5),
//            note("F", 5),
//            note_sharp("F", 5),
//            note("G", 5),
//            note_sharp("G", 5),
//            note("A", 5),
//            note_sharp("A", 5),
//            note("B", 5),
//            note("C", 6),
//            note_sharp("C", 6),
//            note("D", 6),
//        ],
//    );
//    h.insert(
//        'G',
//        [
//            note("G", 4),
//            note_sharp("G", 4),
//            note("A", 4),
//            note_sharp("A", 4),
//            note("B", 4),
//            note("C", 5),
//            note_sharp("C", 5),
//            note("D", 5),
//            note_sharp("D", 5),
//            note("E", 5),
//            note("F", 5),
//            note_sharp("F", 5),
//            note("G", 5),
//            note_sharp("G", 5),
//            note("A", 5),
//            note_sharp("A", 5),
//        ],
//    );
//    h.insert(
//        'D',
//        [
//            note("D", 4),
//            note_sharp("D", 4),
//            note("E", 4),
//            note("F", 4),
//            note_sharp("F", 4),
//            note("G", 4),
//            note_sharp("G", 4),
//            note("A", 4),
//            note_sharp("A", 4),
//            note("B", 4),
//            note("C", 5),
//            note_sharp("C", 5),
//            note("D", 5),
//            note_sharp("D", 5),
//            note("E", 5),
//            note("F", 5),
//        ],
//    );
//    h.insert(
//        'A',
//        [
//            note("A", 3),
//            note_sharp("A", 3),
//            note("B", 3),
//            note("C", 4),
//            note_sharp("C", 4),
//            note("D", 4),
//            note_sharp("D", 4),
//            note("E", 4),
//            note("F", 4),
//            note_sharp("F", 4),
//            note("G", 4),
//            note_sharp("G", 4),
//            note("A", 4),
//            note_sharp("A", 4),
//            note("B", 4),
//            note("C", 5),
//        ],
//    );
//    h.insert(
//        'E',
//        [
//            note("E", 3),
//            note("F", 3),
//            note_sharp("F", 3),
//            note("G", 3),
//            note_sharp("G", 3),
//            note("A", 3),
//            note_sharp("A", 3),
//            note("B", 3),
//            note("C", 4),
//            note_sharp("C", 4),
//            note("D", 4),
//            note_sharp("D", 4),
//            note("E", 4),
//            note("F", 4),
//            note_sharp("F", 4),
//            note("G", 4),
//        ],
//    );
//    h
//});
