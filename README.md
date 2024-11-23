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
- parse chords / lyrics
- "fixup" mode (replace unknown chars with rest), and comment unparseable lines
- bend parsing shouldn't be that hard now
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
