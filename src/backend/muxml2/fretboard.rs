use std::{cell::RefCell, collections::HashMap};

use crate::backend::errors::{backend_error::BackendError, diagnostic::Diagnostic};

use super::MuxmlNote;
use anyhow::bail;
use once_cell::unsync::Lazy;

fn note(note: char, octave: u8) -> MuxmlNote {
    MuxmlNote {
        step: note,
        octave,
        sharp: false,
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

#[allow(clippy::declare_interior_mutable_const)]
pub const STRING_BASE_NOTES: Lazy<HashMap<char, MuxmlNote>> = Lazy::new(|| {
    let mut h = HashMap::new();

    h.insert('e', note('E', 5));
    h.insert('d', note('D', 5));
    h.insert('B', note('B', 4));
    h.insert('G', note('G', 4));
    h.insert('D', note('D', 4));
    h.insert('A', note('A', 3));
    h.insert('E', note('E', 3));
    h
});

thread_local! {
  static NOTE_CACHE: RefCell<HashMap<(char, u16), MuxmlNote>> =  {
    let mut h = HashMap::new();

    h.insert(('e',0), note('E', 5));
    h.insert(('d',0), note('D', 5));
    h.insert(('B',0), note('B', 4));
    h.insert(('G',0), note('G', 4));
    h.insert(('D',0), note('D', 4));
    h.insert(('A',0), note('A', 3));
    h.insert(('E',0), note('E', 3));

    RefCell::new(h)
  };
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
