# Music as code (`src/audio/songs/*.rs`)

One song = one Rust file. There is no JSON, no generator and nothing to
keep in sync: songs compile with the crate and are checked by the same
native unit tests as everything else (`cargo test audio`). There are TWO
ways to write one, and both produce the same `SongSpec`:

* with the builder layer in `src/audio/compose.rs` — composable functions
  (riffs, sections, an arrangement). The seven BRIEFED tracks of the
  soundtrack are written this way (`docs/music/TRACKS.md`; the genre guides
  next to it say what each style must convey and how). The builders cover
  the CLASSIC part of the format: four melodic lanes, one kick / hat /
  snare lane, a `Wave` per voice, per-channel levels, `.ducked()`;
* as `const` section literals against the FULL v2 format (below): the
  eleven songs listed after the briefed ones (`insert_coin.rs` …
  `blood_engine.rs`; `sodium_lights.rs` is the tour — ties, velocity and
  chord lanes, KEYS + PERC, stereo voices, echo + hall sends). They are
  tracker-listed (`?viz` → MUSICS) and have NO ROLE in the game yet; they
  are 27–80 s loops where the briefed tracks are 2–4.5 min arrangements.

The playback side is plain data: a finished song is a `SongSpec` — key,
tempo, a `Voice` per melodic lane, the song-level bus settings, an ordered
`&'static [Section]` — and the sequencer (`songs.rs`), the bake queue
(`music_keys`), the `?viz` TRACKER page and the role pickers (`title_song`
/ `song_for_floor` / `ending_song`) consume it. A built song's `build()`
runs once (memoized in a `OnceLock`) and leaks its lanes into the static
slices.

## The format (v2) — `songs.rs` + `voice.rs`

* **Seven channels** (`CHANNEL_NAMES`): five melodic lanes `BASS LEAD PAD
  ARP KEYS` (scale degrees) + two percussion lanes `DRUMS PERC` (the same
  kit: `Drum::{Kick, Hat, Snare, Clap, OpenHat, Tom, Rim, Crash}` — two
  lanes so a hat can ride over a kick). Lanes loop inside a section; a
  section is as long as its longest NOTE lane.
* **Sentinels**: `REST`, and `HOLD` = a TIE — a note's length is 1 + the
  `HOLD`s that follow it (wrapping around the looping lane). The engine
  holds the note at its peak for the tied steps, then plays the lane's
  usual pluck: untied notes sound exactly as before.
* **Velocity lanes** (`bass_vel` … `perc_vel`): one `0..=MAX_VEL` (9) per
  step, looping; empty = full, `0` = skip the note. Linear amplitude,
  multiplied by the section's per-channel `level` — both PLAY-time gains:
  velocity never grows the bake set.
* **Chord lanes** (`*_chord`): a `Chord` voicing per step (`Single Octave
  Power Triad Sus2 Sus4 Seventh Add9 Inv1 Inv2 Open` = in-key degree
  offsets), read where a note starts; empty = `Triad` for the pad,
  `Single` otherwise. Each partial plays at 1/√n of the lane's level.
* **`Voice`** — one per melodic lane, `const fn` builders: `Voice::mono /
  panned(wave, pan) / wide(wave, pan, detune, width) / stack(wave, pan,
  detune, width, unison ≤ 7)`, then `.with_filter(cutoff, peak, attack,
  decay, q)` (a per-note LOWPASS envelope: attack > 0 = bloom, decay > 0 =
  wow), `.with_vibrato(rate, cents, delay)`, `.with_env(attack,
  gate_steps)` (overrides the lane's pluck), `.with_drive(0..1)`,
  `.with_echo(send)`, `.with_reverb(send)`, `.with_sub(level)`,
  `.with_glide(seconds)` (portamento into a LEGATO note — one that starts
  where the previous one ends). `Wave` = the four raw shapes, `Noise`
  (risers, wind: the degree is ignored) and the three darksynth PRESETS;
  a preset brings its own graph, so unison / filter / vibrato / glide do
  not apply to it (envelope, ties, chords, sub, pan, drive, sends do).
* **Song-level**: `swing` (0..1: the odd sixteenths late by up to a third
  of a step — `swing_delay`), `humanize` (seconds, ≤ 0.02: every note but
  the kicks, never into the clock's past), `sidechain: Sidechain { depth,
  release_beats }` (armed per section by `Section::duck`), `echo: Echo {
  steps, feedback, tone }`, `sweep` (0..1: depth of the per-bar bus wah),
  `intensity`, and `melodic_gain` (see "the lane channels" below).
* **`MusicKey`** = what gets baked: `Note { lane, degree, len, chord, from
  }` | `Drum(_)`. Pitch, tied LENGTH and voicing are baked, velocity is
  not; `from` (the legato origin) enters the key only on a gliding voice.

`docs/music/` has the genre guides; `voice_summary` / the tracker's VOICES
panel show what a song's instruments are made of.

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

To add or replace a BRIEFED track: create the file, `pub mod <name>;` it in
`src/audio/songs.rs`, add its `spec()` to the first `BRIEFED_SONGS` entries
of `SONGS` (and bump both counts), give it a ROLE (`title_song`, a
`song_for_floor` range, or `ending_song` — a briefed song with no role
fails `soundtrack_roles_follow_the_briefs`), add its target length to
`tracks_run_the_briefed_length`, and put its brief in
`docs/music/TRACKS.md`. A tracker-only song goes after them (bump
`SONG_COUNT`). The tests in `songs.rs` enforce the rest.

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

Compose against the instrument you have. `audio/engine` plays a song as
baked one-shots per `MusicKey` through the lane channels and the music
bus. What the `compose` BUILDERS can express (the v2 format above goes
further — until the builders learn it, a song that needs ties, velocity
lanes, the big kit or stereo voices is written as `const` literals):

* **Note lengths per channel**, in steps: bass ≈ 1.9, lead ≈ 0.9, arp ≈
  0.7, keys ≈ 1.2, pad = 4 (one beat; the dark pad's attack takes the
  first step). Built songs have no ties: a long note is a RETRIGGER — a
  pad "held" for a bar is `0 . . . 0 . . . 0 . . . 0 . . .`, a droning
  lead is the same degree on every step at a low velocity
  (`thermal_mass.rs`'s `drone`).
* **The pad blooms a triad** (root + third + fifth of the scale) per note:
  chord = one degree. Extensions (9ths, 11ths) come from the arp or lead
  sitting on top (`coast_home.rs`'s raindrops over the Am bed).
* **Three drums** in `hits()`: kick, hat, snare — "metallic ticks" are
  sparse hats, a stutter is `ssss`, a burst is `hhhh`.
* **The grid is straight 16ths** (built songs: `swing` 0, `humanize` 0); a
  12-step lane against 16-step bars makes the grid breathe.
* **Plain centred voices** (`Voice::mono(wave)`): no pan, drive or sends;
  the `Supersaw` / `DarkPad` presets carry their own spread.
* **The bus, per song**: `intensity` sets the level and the per-bar
  lowpass sweep's peak (higher = darker, tighter); a section's
  `.ducked()` pumps the melodic lanes on every kick through the song's
  `Sidechain` — the builder writes depth `DUCK_DEPTH` (0.65: a dip to
  0.35) and a release of `DUCK_RECOVERY` (0.3 s) at the song's tempo.
* **Dynamics** are per channel per section (`.vel` → `Section::level`).
  A fade is a sequence of thinner sections.
* **Bake budget**: each song's voice set (`music_keys`) must stay ≤ 96
  keys — distinct (degree, tied length, voicing) per melodic lane + the
  kit pieces used. The briefed tracks use 15–34, the v2 songs up to 53.

## The lane channels (what is NOT baked)

```text
 note ─► lane panner ─► drive ─┬───────────────────────────► ducker ─► bus ─► lowpass ─► soft-clip ─► out
                               ├─ echo send ─► delay ─► tone ─► return ──┤
                               │                 ▲           └─ feedback ─┘
                               └─ verb send ─► convolver (hall) ─► return ─┘
 drum ─────────────────────────────────────────────────────────────────► bus
```

A baked note is DRY: pan, drive, the echo / hall sends, the duck and the
per-bar sweep are live, per lane, and re-pointed at every song change
(`AudioEngine::apply_voices`). Two consequences worth knowing:

* every lane plays through an equal-power `StereoPannerNode`, which puts a
  CENTRED lane 3 dB under the drums (they enter the bus directly).
  `SongSpec::melodic_gain` is the bake-time make-up: the builders write
  `SQRT_2` (a plain centred lane is exactly as loud as before the lane
  graph existed — measured), the `const`-literal songs, mixed with the
  panners in place, say `1.0`;
* the bus's safety limiter is a static soft-clip (a wire under 0.7), NOT a
  `DynamicsCompressorNode` — that node's automatic make-up gain made the
  whole soundtrack ×1.9 louder against the SFX when it was tried
  (`docs/HISTORY.md`). The music level is ONE constant: `MUSIC_GAIN`.

A note that is not baked yet plays as a SKETCH (`AudioEngine::sketch`: one
plain oscillator per partial — no stack, filter, vibrato or sub), so the
live fallback's cost never scales with how rich a voice is.

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

A `Section` with `duck` set (`.ducked()` in the builders; the default of a
`const` literal) pumps: every sounding kick pulls the ducker down by the
song's `Sidechain::depth` in 4 ms and releases it exponentially (~95 %
back after `release_beats`) — the pure curve `voice::duck_level(depth,
tau, dt)`, which is also how a retriggering kick picks the release up
where it is (host-tested); the scheduler programs it as gain automation on
the duck node (`AudioEngine::duck`: set, a 4 ms ramp down, a
`setTargetAtTime` release). Drums bypass the node and never duck
themselves; baked note buffers stay duck-free (the duck, like the per-bar
filter sweep, is live-bus automation). No effect when the song's
`sidechain` is `Sidechain::OFF`.

## Invariants (`cargo test audio`)

* every section's longest lane is whole bars (`sections_are_bar_aligned`);
* every song's voice set enumerates — over the SECTION's steps, so a lane
  looping under a chord lane of another length is covered
  (`looping_lanes_meet_every_chord_lane_step`) — and stays within the
  pre-render budget of 96 keys (`music_voice_sets_are_small_and_complete`
  — run with `--nocapture` for per-song counts);
* the ROLES follow the briefs: the title is "Neon Checksum", the ending
  "Coast Home", every floor id maps to its track, every BRIEFED song has
  a role, the ending is the calmest of them
  (`soundtrack_roles_follow_the_briefs`);
* every briefed track runs its briefed length (±20 s, within 1:30–5:00 —
  `tracks_run_the_briefed_length`, `--nocapture` for every song's
  seconds);
* the format reads as documented: ties and legato origins
  (`ties_extend_the_note_they_follow`,
  `legato_origin_needs_touching_different_notes`), velocity / chord lanes,
  the kit on both percussion lanes, tracker cells, bake lengths
  (`bake_lengths_cover_the_note`); every song's settings are inside the
  ranges the engine assumes
  (`songs_are_well_formed_and_floor_mapping_is_listed`);
* the duck is a genre marker: the darksynth-led tracks pump (every kicked
  section of the pure ones), the wave-led ones never do
  (`the_duck_follows_the_genre`);
* `song_for_floor` is total and only names listed songs
  (`songs_are_well_formed_and_floor_mapping_is_listed`);
* the duck curve dips and recovers monotonically, swing delays only the
  off sixteenths (`voice.rs`), and the compose combinators are unit
  tested in `compose.rs` (including seeded-variation determinism);
* the ENGINE's graphs (`audio/engine/tests.rs`, through the recording
  mock): the lane channels are wired in order, a song change re-points
  them, wide voices bake stereo and ties hold their peak, built songs make
  the centre-pan law up, the live sketch is bounded, the scheduler is
  never in the clock's past and ducks exactly in the ducked sections —
  next to the older "every note of every song bakes a sound graph that
  fits its length".
