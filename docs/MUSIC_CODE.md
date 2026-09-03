# Music as code (`src/audio/songs/*.rs`)

One song = one Rust file of composable functions. There is no JSON, no
generator and nothing to keep in sync: songs are authored directly with
the builder layer in `src/audio/compose.rs`, compile with the crate, and
are checked by the same native unit tests as everything else
(`cargo test audio`). The seven tracks of the soundtrack and their briefs
are in `docs/music/TRACKS.md`; the genre guides next to it say what each
style must convey and how.

The playback side is plain data: a finished song is a `SongSpec` — key,
tempo, per-voice `Wave`, an ordered `&'static [Section]` — and the
sequencer (`songs.rs`), the bake queue (`music_keys`), the `?viz` TRACKER
page and the role pickers (`title_song` / `song_for_floor` /
`ending_song`) consume it. A song file's `build()` runs once (memoized in a
`OnceLock`) and leaks its lanes into the static slices.

## The shape of a song file

The compiled reference is `src/audio/songs/service_corridor.rs` (the
house-genre track: riff, layers, refrain as a function, bridge,
breakdown). In miniature:

```rust
//! What the track conveys, its genre mix, key, tempo, length — and the
//! genre checklist, ticked, so the intent is reviewable.

use super::super::compose::*;
use super::{SongSpec, Wave, PHRYGIAN};
use std::sync::OnceLock;
use Intensity::{Cool, Hot, Warm};

/// A riff is a FUNCTION. `steps` parses tokens: `.` = rest, an integer =
/// a scale degree (0 root, 7 an octave up), `|` cosmetic, one bar per line.
fn riff() -> Lane {
    steps("0 . 0 0 . 0 . 0 7 . 0 0 1 . 0 .")
}

/// Combinators shape riffs: `transpose` / `repeat` / `cat` / `then` /
/// `every_other_bar` / `stretch` / seeded `sparsify`.
fn chords() -> Lane {
    cat([bed(0), bed(0), bed(0), bed(1)])
}

/// A section is named parts. Same-channel parts OVERLAY; `.vel()` scales a
/// channel; `.ducked()` arms the sidechain pump.
fn drive() -> SectionSpec {
    section("drive", [bass(repeat(riff(), 8)), pad(repeat(chords(), 2)).vel(0.8),
                      drums(hits("k.h.k.s.k.h.k.s.").repeat(8))])
        .ducked()
}

/// The refrain is ONE function — call it again and it comes back; give it
/// an `Intensity` and it comes back HOTTER.
fn refrain(heat: Intensity) -> SectionSpec { /* drive() + the lead, more with heat */ }

/// The song is an ARRANGEMENT of named sections — the form reads at a
/// glance.
fn build() -> SongSpec {
    song("Service Corridor", Key::new(E1, PHRYGIAN), 118.0)
        .waves(Wave::DrivenBass, Wave::Supersaw, Wave::DarkPad, Wave::Square)
        .intensity(0.95)
        .arrange([open(), build_up(), refrain(Cool), drive(), half_time(),
                  refrain(Warm), bridge(), breakdown(), drive(), refrain(Hot),
                  half_time(), outro()])
        .build()
}

pub fn spec() -> SongSpec {
    static SPEC: OnceLock<SongSpec> = OnceLock::new();
    *SPEC.get_or_init(build)
}
```

Songs can share material across files: `neon_checksum::motif()` is `pub`
and the ending quotes it (`coast_home.rs`: `transpose(stretch(call, 2),
-7)` — half speed, an octave down).

To add or replace a track: create the file, `pub mod <name>;` it in
`src/audio/songs.rs`, add its `spec()` to `SONGS` (and bump `SONG_COUNT`),
give it a ROLE (`title_song`, a `song_for_floor` range, or `ending_song`
— a listed song with no role fails `soundtrack_roles_follow_the_briefs`),
add its target length to `tracks_run_the_briefed_length`, and put its
brief in `docs/music/TRACKS.md`. The tests in `songs.rs` enforce the rest.

## The atoms

* **`Key::new(root_hz, scale)`** — root constants `C1..B1` (12-TET) and
  the named scales (`MINOR`, `DORIAN`, `HARMONIC_MINOR`, `PHRYGIAN`,
  `PHRYGIAN_DOMINANT`, `LOCRIAN`) live in `songs.rs`/`compose.rs`.
  Degrees are SCALE degrees: on a 7-note scale 7 is the octave, and
  `transpose(riff, 4)` in Locrian is a tritone shift (`signal_rot.rs`).
* **`Lane`** (melodic) — from `steps("0 . 3 . | 5 . 3 .")` (whitespace and
  `|` cosmetic; multi-line raw strings read one bar per line) or
  `Lane::from(vec![...])`. `&str` converts implicitly wherever a lane is
  expected: `bass("0 . 3 .")` just works.
* **`DrumLane`** — from `hits("k.h.k.h.")`: `.` silent, `k` kick, `h` hat,
  `s` snare. One drum per step: a snare on 2 and 4 REPLACES the kick there
  (`k.h.k.s.` is the four-on-the-floor idiom).
* **Combinators** — `transpose(riff, +3)` (scale degrees; rests stay),
  `repeat(riff, 4)`, `cat([a, b, c])` / `a.then(b)` (`cat_hits` for
  drums), `every_other_bar(a, b)` (rest-padded to align), `stretch(riff,
  2)` (every step becomes two — a motif at half speed), `sparsify(riff,
  seed, 0.3)` (seeded xorshift; same seed = same holes).
* **Parts** — `bass(..)`, `lead(..)`, `pad(..)` (notes bloom into triads at
  playback), `arp(..)`, `drums(..)`; `with_velocity(part, 0.8)` /
  `part.vel(0.8)` scales its CHANNEL for the section (applied at schedule
  time — costs nothing in the bake budget). Two parts on one channel
  overlay; their velocities MULTIPLY, so scale one of them.
* **`section(label, [parts])`** — same-channel parts overlay (shorter
  loops under longer, later non-rest steps win); `.ducked()` arms the
  sidechain. Lanes inside a section may differ in length — a section plays
  as long as its LONGEST lane and shorter lanes loop inside it (a 12-step
  lane phases 3-against-4 over 16-step bars) — but the longest lane must
  be whole bars (enforced by `sections_are_bar_aligned`).
* **`song(name, key, bpm)`** — `.steps_per_beat(4)` (default), `.waves(..)`
  per voice, `.intensity(0.5 lounge .. 1.2 boss)`, `.arrange([...])`,
  `.build()`.

## What the engine can and cannot do

Compose against the instrument you have. `audio/engine.rs` plays a song
as baked one-shots per (channel, degree) through a music bus; the
consequences for writing:

* **Note lengths are fixed per channel**, in steps: bass ≈ 1.9, lead ≈
  0.9, arp ≈ 0.7, pad = 4 (one beat; the dark pad's attack takes the
  first step). There is no sustain: a long note is a RETRIGGER — a pad
  "held" for a bar is `0 . . . 0 . . . 0 . . . 0 . . .`, a droning lead
  is the same degree on every step at a low velocity
  (`thermal_mass.rs`'s `drone`). Lead melodies are staccato by nature —
  write them as 8ths and quarters with air, not as ties.
* **The pad blooms a triad** (root + third + fifth of the scale) per note:
  chord = one degree. Extensions (9ths, 11ths) come from the arp or lead
  sitting on top (`coast_home.rs`'s raindrops over the Am bed).
* **Three drums**: kick, hat, snare. No clap, open hat, tick or roll
  voice — "metallic ticks" are sparse hats, a stutter is `ssss`, a burst
  is `hhhh`.
* **The grid is straight 16ths.** No swing, no triplets, no off-grid
  placement; a 12-step lane against 16-step bars is the one way to make
  the grid breathe.
* **No pitch tricks**: no portamento, no per-voice detune (beyond the
  `Supersaw` / `DarkPad` presets' own spread), no LFO. "Detune beating"
  is approximated by the b2 held against the root.
* **The bus, per song**: `intensity` sets the level and the per-bar
  lowpass sweep's peak (higher = darker, tighter); a section's
  `.ducked()` pumps the melodic bus on every kick — depth
  (`DUCK_FLOOR` 0.35) and recovery (0.3 s) are constants, not per
  section. There is no reverb send on the music bus (the reverb belongs
  to the SFX buses), so "space" is arrangement: rests, low velocities.
* **Dynamics** are per channel per section (`.vel`) — there is no
  per-step velocity and no automation. A fade is a sequence of thinner
  sections.
* **Bake budget**: each song's voice set (`music_keys`) must stay ≤ 64
  keys — distinct degrees per melodic channel + up to three drums. The
  soundtrack's tracks use 15–34.

## Voices (`Wave`)

The four raw shapes (`Sine`, `Square`, `Sawtooth`, `Triangle`) plus the
darksynth presets, synthesized in `engine.rs` from small node graphs and
baked per pitch exactly like the raw shapes:

* **`Supersaw`** — five saws detuned across ±12 cents, center loudest.
* **`DrivenBass`** — saw + sub-octave square driven into a waveshaper soft
  clip (heavy knee), envelope after the clipper.
* **`DarkPad`** — a ±7-cent saw pair through a fixed ~900 Hz lowpass with a
  slow attack.

## The sidechain duck

A `Section` built with `.ducked()` pumps: every sounding kick step snaps
the melodic stage of the music bus to `DUCK_FLOOR` and ramps it back to 1
over `DUCK_RECOVERY` seconds — the pure curve `songs::duck_gain(dt)`
(host-tested), reproduced 1:1 by the scheduler as gain automation on the
duck node. Drums bypass the node and never duck themselves; baked note
buffers stay duck-free (the duck, like the per-bar filter sweep, is
live-bus automation).

## Invariants (`cargo test audio`)

* every section's longest lane is whole bars (`sections_are_bar_aligned`);
* every song's voice set enumerates and stays within the pre-render budget
  of 64 keys (`music_voice_sets_are_small_and_complete` — run with
  `--nocapture` for per-song counts);
* the ROLES follow the briefs: the title is "Neon Checksum", the ending
  "Coast Home", every floor id maps to its track, every listed song has a
  role, the ending is the calmest track
  (`soundtrack_roles_follow_the_briefs`);
* every track runs its briefed length (±20 s, within 1:30–5:00 —
  `tracks_run_the_briefed_length`, `--nocapture` for the per-track
  seconds);
* the duck is a genre marker: the darksynth-led tracks pump (every kicked
  section of the pure ones), the wave-led ones never do
  (`the_duck_follows_the_genre`);
* `song_for_floor` is total and only names listed songs
  (`songs_are_well_formed_and_floor_mapping_is_listed`);
* the duck curve dips, recovers monotonically and retriggers
  (`duck_curve_dips_and_recovers`), and the compose combinators are unit
  tested in `compose.rs` (including seeded-variation determinism).
