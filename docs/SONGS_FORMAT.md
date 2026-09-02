# Song format (`songs/*.json`)

One JSON file per tracker song in `songs/<name>.json`; `songs/index.json` lists
them in `SONGS` order (the `?viz` MUSICS tracker's list — floors pick theirs
through `song_for_floor` in `src/audio/songs.rs`). This JSON is the single
source of truth: `tools/gen_songs.py` (Python stdlib only — no crates)
generates `src/audio/songs_data.rs` (checked in) which the engine compiles as
static data. `make gen-songs` regenerates it; `make check-songs` (part of
`make verify`) validates the JSON and fails if the generated file is stale.

The types the data describes live in `src/audio/songs.rs` (`SongSpec`,
`Section`, `Drum`, `Wave`, the named scales) — host-compiled, so the songs are
unit-tested natively (`cargo test audio::`): every section is whole bars,
every voice a song can schedule is enumerated for the bake queue, the floor
mapping only names listed songs.

```jsonc
// songs/index.json
{
  "songs": [
    { "file": "insert_coin.json", "name": "Insert Coin" },   // name is optional here,
    { "file": "neon_lounge.json", "name": "Neon Lounge" }    // checked against the file when given
  ]
}
```

The file stem must be `snake_case` (`[a-z][a-z0-9_]*`): it becomes the Rust
const (`insert_coin.json` → `pub const INSERT_COIN: SongSpec`, which
`song_for_floor` references by name).

```jsonc
// songs/razor_circuit.json
{
  "name": "Razor Circuit",                   // shown in the tracker; unique across songs
  "notes": [                                 // OPTIONAL free text, emitted as comments
    "SONG 9 — driving darksynth combat. E natural minor, 126 BPM. ..."
  ],
  "root": 41.2,                              // tonic frequency, Hz (> 0)
  "root_note": "E1",                         // OPTIONAL documentation (a comment in the Rust)
  "scale": "minor",                          // a named scale (below) or explicit semitone
                                             // offsets from the root, e.g. [0, 2, 3, 5, 7, 8, 10]
  "bpm": 126.0,                              // tempo (> 0)
  "steps_per_beat": 4,                       // sequencer resolution (4 = sixteenths);
                                             // a bar is 4 beats = 4 * steps_per_beat steps
  "waves": { "bass": "square", "lead": "sawtooth", "pad": "sawtooth", "arp": "square" },
                                             // oscillator per melodic voice:
                                             // sine | square | sawtooth | triangle
  "intensity": 0.9,                          // punch / loudness feel (~0.5 lounge .. ~1.2 boss)

  "patterns": {                              // OPTIONAL named lanes shared by several sections
    "razor_pad": [
      "0 . . . . . . . 0 . . . . . . .",
      "5 . . . . . . . 5 . . . . . . .",
      "2 . . . . . . . 2 . . . . . . .",
      "6 . . . . . . . 6 . . . . . . ."
    ]
  },

  "sections": {                              // the blocks of the arrangement, keyed by a
    "groove": {                              // snake_case id (also the section's label
      "label": "groove",                     // unless "label" overrides it — OPTIONAL)
      "bass": ". . 0 . . . 0 . . . 0 . . . 0 .",
      "lead": ".",
      "pad": "@razor_pad",                   // "@name" = use the named pattern
      "arp": [
        "7 9 11 14 11 9 7 9 7 9 11 14 11 9 7 9",
        "12 14 16 19 16 14 12 14 12 14 16 19 16 14 12 14"
      ],
      "drums": "k.h.k.h.k.h.k.hh"
    }
  },
  "order": ["intro", "intro", "groove", "groove", "hook", "hook", "breakdown", "groove", "hook", "hook"]
                                             // the arrangement: section ids, played back to
                                             // back then looped as a whole; a section may
                                             // appear any number of times
}
```

## Lanes

Every section has exactly the five lanes `bass`, `lead`, `pad`, `arp`,
`drums`. A lane is written as TOKENS, one per sequencer step:

* **Melodic lanes** (`bass`, `lead`, `pad`, `arp`) are whitespace-separated
  tokens: `.` is a rest, an integer is a SCALE DEGREE — `0` the root, `1` the
  next scale note up, `7` an octave up (for a 7-note scale), negative degrees
  (`-2`) drop below the root. Notes stay in key whatever the root / scale.
  The `pad` lane blooms every note into a triad (root + third + fifth of the
  scale) with a slow attack.
* **The drum lane** is one CHARACTER per step, no separators needed:
  `.` silent, `k` kick, `h` hat, `s` snare.
* A lane value is either one string or a LIST of strings, which are simply
  concatenated (one bar per line reads best). `|` is a purely visual bar
  separator, ignored by the parser. Whitespace is ignored in drum lanes.
* `"@name"` uses the pattern `patterns[name]` instead (a pattern is parsed
  as whichever lane kind references it; it may not be used as both a drum and
  a melodic lane).

Lanes inside a section may differ in length: a short lane LOOPS under a
longer one (a 16-step bass repeats under a 32-step lead; Cold Storage's
12-step motif phases 3-against-4 over its 64-step sections). A section's
playable length is its LONGEST lane, and that length must be whole bars
(`4 * steps_per_beat` steps) — the sequencer moves to the next section when
the longest lane ends, so a ragged one would shift everything after it off
the beat grid. `"."` alone is a valid 1-step silent lane (an empty string is
an empty lane: also silent).

## Named scales

`minor` (aeolian), `dorian`, `harmonic_minor`, `phrygian`,
`phrygian_dominant`, `locrian` — semitone offsets in `src/audio/songs.rs`.
An explicit list (`[0, 2, 3, ...]`, values 0..11) works too.

## Validation (`make check-songs`)

Unknown keys, a missing lane, a bad token, an unresolved `@pattern`, a
section or pattern never used, an `order` entry naming no section, a section
whose longest lane is not whole bars, an unknown wave / scale, a duplicate
song name or file, or a stale `src/audio/songs_data.rs` all fail the check
with a message naming the file / section / lane.
