# guitar_tab
Tools to wrangle guitar tabulature notation.

## Features
- can produce a midi file from a tab file in nanoseconds using the midi backend, suitable for playing tab files in real time (**midi** backend)
- can translate a tab file to classical music notation in the .musicxml format (**muxml**, **muxml2** backends)

- tries to work around and fix bad tabs on the fly when translating
- user friendly error reports and diagnostics
- generates simple and beautiful scores, close to what you could achieve manually
- well documented and exploration-friendly CLI
- generally well optimized for performance

## TODO
- add vibrato
- add slides
- new parser with smarter fixup

- streaming midi for live playback
this works well already for normal size tabs (like a guitar solo or something)
but for larger tabs you have to skip a frame (~10ms) to parse and export the tab to a midi,
which will be parsed back when playing using some other program anyway.
Try to do this ourselves, by having a streaming parser, which will yield to the backend after
parsing a Part (~100us at most). The backend then would have spun off a thread that writes to a
MIDI file descriptor.
I think we can assume that parsing a Part will never take longer than playing the Part. If the
Part has at least one rest, it will play for 100ms and we'll have parsed the next one already
by then.

- fix the GUI and merge it into this workspace
- allow per-part retuning (always use the current specified string name instead of global)
- "fixup" mode (replace unknown chars with rest), and comment unparseable lines
    - also accept no string names

## DONE
- accept dead notes!
- desktop / mobile app
- write tests for backends
- Display on measure for better errors
- don't hardcode frets
- accept drop d tuning
- new muxml backend: use chords, don't write multiple tracks
- MuXML backend: merge rests generated as an ast-optimization step
- a lot of perf improvements
    - muxml2 right now is just a bunch of string formatting which slows the backend down, so write the ast in one go
- rethink error and diagnostic handling, right now diagnostics are part of an error but there is no real reason for this.
- bend parsing shouldn't be that hard now
