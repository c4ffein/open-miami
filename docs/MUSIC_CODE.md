# Music as code (`src/audio/songs/*.rs`)

One song = one Rust file. There is no JSON, no generator and nothing to
keep in sync: songs compile with the crate and are checked by the same
native unit tests as everything else (`cargo test audio`). There are TWO
ways to write one, and both produce the same `SongSpec`:

* with the builder layer in `src/audio/compose.rs` — composable functions
  (riffs, sections, an arrangement), which covers the WHOLE format. The
  seven BRIEFED tracks of the soundtrack are written this way
  (`docs/music/TRACKS.md`; the genre guides next to it say what each style
  must convey and how) — with the classic instrument so far —, and so is
  `sodium_lights.rs`, THE TOUR of the v2 side (ties, velocity and chord
  lanes, KEYS + PERC, the big kit, stereo voices, echo + hall sends);
* as `const` section literals against the raw format (below): the ten
  other songs listed after the briefed ones (`insert_coin.rs` …
  `blood_engine.rs`). Both kinds are tracker-listed (`?viz` → MUSICS);
  the eleven v2 songs have NO ROLE in the game yet and are 27–80 s loops
  where the briefed tracks are 2–4.5 min arrangements.

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
  (risers, wind: the degree is ignored), the three darksynth PRESETS — a
  preset brings its own graph, so unison / filter / vibrato / glide do
  not apply to it (envelope, ties, chords, sub, pan, drive, sends do) —
  the three COMPUTED strings and the Reese / FM voices (below).
  `.with_bend(semitones, seconds)` starts EVERY note that far off pitch and
  arrives over `seconds` (the "yoy" of a lead, a laser dive; a legato glide
  wins where it applies); `.with_wobble(steps, cutoff, peak, q)` is THE
  WOBBLE — a resonant lowpass swinging `cutoff` → `peak` once per `steps`
  steps (`2.0` = an eighth, `1.0` = a sixteenth: the drop's grind), a sine
  from the midpoint, restarting with every note (it is baked, as a wobble
  bass is played). Both work on node-built and computed voices alike. The
  wobble's RATE can change per step: a `*_wob` lane (`.wobbling("2 2 1
  1")` on the part — steps per wobble, `.` = the voice's own) is read where
  a note starts and enters the bake key on a wobbling voice.
* **Section RAMPS** (`Section::ramps`, `.ramps([..])` in the builders):
  start → end movements of a lane's LIVE channel across one section —
  `Ramp::cutoff(lane, from, to)` (the lane's own lowpass, open at rest),
  `Ramp::echo` / `Ramp::reverb` (the sends), `Ramp::pan`, `Ramp::level`
  (a fade). Automation on the persistent nodes: nothing to bake, any
  voice; a parameter no ramp names is restored to the voice's value at
  every section start. `Ramp::cutoff(..).over(3)` runs the ramp across
  three sections from the one it is listed in — the sections it runs
  through leave that parameter alone — so a sixty-second swell is ONE
  ramp. A filter opening over sixteen bars, a lead sinking
  into the hall, an outro fading: this is where slow movement lives —
  `low_tide.rs` (eight sections of eight bars, 2:34) is built on nothing
  else: the pad's and the arpeggio's lowpasses open across whole
  sections, the arpeggio fades in over the first twenty seconds, the lead
  sinks into the hall as the drums leave, the pad fades out at the end.
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

## Computed voices (`audio/dsp.rs`)

`Wave::Guitar` (a picked steel string), `Wave::BassGuitar` (a fingered
bass), `Wave::Violin` (a bowed string), `Wave::Reese` (two saws nine cents
apart folded through a soft clip — the dubstep / DnB growl; put a wobble
on it) and `Wave::Fm` (a sine carrier phase-modulated at its own pitch by
an index decaying from a growl to a tone) are not built from Web Audio
nodes: their samples are computed in Rust and copied into the bake buffer
(`AudioEngine::computed_bake`, synchronous — no offline context). The
plucked strings are Karplus–Strong loops — a pick-position-notched noise
burst circulating through a fractional (allpass) delay line and a one-pole
loss filter, decaying to −60 dB over the model's `t60` at any pitch, plus
a body resonance; a node graph cannot do this because a feedback loop
through nodes has a minimum delay of one render quantum (≈ 2.7 ms), which
caps a string near 370 Hz. The violin is a band-limited (polyBLEP) saw
with bow noise through three body resonances, under a swell that is never
shorter than 60 ms.

What the `Voice` builders do to a computed wave: `with_env` (attack /
gate — a guitar note needs a gate of ~5–6 steps to ring), `with_filter`
(a state-variable lowpass with the same envelope), `with_vibrato` and
`with_glide` (the pitch curve is per block), `with_sub`, pan / drive /
sends; chords play every partial (a plucked chord STRUMS, low string
first, 14 ms apart — `dsp::STRUM_SECONDS`); a unison stack (`Voice::wide`
/ `stack`), which bakes STEREO when wide, each oscillator placed
equal-power across the image (Salt Road's violins are wide pairs: a string
section). The
live SKETCH of an unbaked computed note is the usual plain oscillator (a
triangle for the plucked ones, a saw for the violin). Cost: about 7 ms per
second of audio per partial in release (a strummed triad held a bar ≈
20 ms; a wide violin pair held a bar ≈ 20 ms), ONE computed note per frame whatever the pump budget (`baked_sync`); the `salt_road.rs` bake adds ~70 ms of long
tasks to a page load (measured, headless). `salt_road.rs` is the showcase
of the strings: bass guitar, picked + strummed guitar, two violins;
`static_teeth.rs` of the modifiers: two Reeses (an eighth-note and a
sixteenth-note wobble), FM growls, a bending lead, and section ramps
opening the pad through the build, drowning the lead in the break and
fading the outro.

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
* **`Lane`** (melodic) — from `steps("0 _ _ . | 5 . 3 .")` (`.` rest, `_`
  a TIE: the previous note holds through the step; whitespace and `|`
  cosmetic; multi-line raw strings read one bar per line) or
  `Lane::from(vec![...])`. `&str` converts implicitly wherever a lane is
  expected: `bass("0 . 3 .")` just works.
* **`DrumLane`** — from `hits("k.h.k.h.")`: `.` silent, `k` kick, `h` hat,
  `s` snare, `c` clap, `o` open hat, `t` tom, `r` rim, `x` crash. One drum
  per step per lane: a snare on 2 and 4 REPLACES the kick there
  (`k.h.k.s.` is the four-on-the-floor idiom) — what has to hit TOGETHER
  goes on the second lane, `perc(..)`.
* **Combinators** — `transpose(riff, +3)` (scale degrees; rests and ties
  stay), `repeat(riff, 4)`, `cat([a, b, c])` / `a.then(b)` (`cat_hits` for
  drums), `every_other_bar(a, b)` (rest-padded to align), `stretch(riff,
  2)` (every step becomes two — a motif at half speed, plucked) /
  `held(riff, 2)` (the same, SUSTAINED: the note then ties), `sustain(riff)`
  (every rest after a note becomes a tie: legato), `sparsify(riff, seed,
  0.3)` (seeded xorshift; same seed = same holes; a dropped note takes its
  ties with it).
* **Velocity + chord lanes** — `accents("9.6.")` (one char a step: a digit
  `0`–`9`, `.` = full, `0` skips the note) and `chords("7 t 9 t")` (`.`
  the lane's default, `s o p t 2 4 7 9 i j w` = `Chord::{Single, Octave,
  Power, Triad, Sus2, Sus4, Seventh, Add9, Inv1, Inv2, Open}`;
  `.each(16)` = one token per BAR). Both loop under the part's lane.
* **Parts** — `bass(..)`, `lead(..)`, `pad(..)` (notes bloom into triads at
  playback), `arp(..)`, `keys(..)`, `drums(..)`, `perc(..)`;
  `with_velocity(part, 0.8)` / `part.vel(0.8)` scales its CHANNEL for the
  section, `.accents(..)` sets its per-step velocity, `.voiced(..)` its
  per-step voicing (all applied at schedule time / in the bake key — the
  accents cost nothing in the bake budget). Two parts on one channel
  overlay; their levels MULTIPLY, so scale one of them; each part's
  accents / voicing follow ITS notes through the overlay.
* **`section(label, [parts])`** — same-channel parts overlay (shorter
  loops under longer, later non-rest steps win); `.ducked()` arms the
  sidechain. Lanes inside a section may differ in length — a section plays
  as long as its LONGEST lane and shorter lanes loop inside it (a 12-step
  lane phases 3-against-4 over 16-step bars) — but the longest lane must
  be whole bars (enforced by `sections_are_bar_aligned`).
* **`song(name, key, bpm)`** — `.steps_per_beat(4)` (default), `.waves(..)`
  (plain centred shapes, bass / lead / pad / arp) or `.voices(..)` (the
  full `Voice` per lane, incl. keys), `.intensity(0.5 lounge .. 1.2
  boss)`, `.swing(0..1)`, `.sidechain(Sidechain::new(depth, beats))`,
  `.echo(Echo::new(steps, feedback, tone))`, `.humanize(s)`, `.sweep(0..1)`,
  `.melodic_gain(..)` (see "the lane channels"), `.arrange([...])`,
  `.build()`.

## What the engine can and cannot do

Compose against the instrument you have. `audio/engine` plays a song as
baked one-shots per `MusicKey` through the lane channels and the music
bus:

* **Note lengths per channel**, in steps: bass ≈ 1.9, lead ≈ 0.9, arp ≈
  0.7, keys ≈ 1.2, pad = 4 (one beat; the dark pad's attack takes the
  first step) — the PLUCK after a note's last step. A tie (`_`) holds the
  note at its peak through the tied steps first; without ties a long note
  is a RETRIGGER (a pad "held" for a bar is `0 . . . 0 . . . 0 . . . 0 . .
  .`), which the briefed tracks still use (`thermal_mass.rs`'s `drone`).
* **The pad blooms a triad** (root + third + fifth of the scale) per note
  by default; `.voiced(..)` changes the voicing per step on any melodic
  lane. Every partial plays at 1/√n of the lane's level.
* **The kit**: kick, hat, snare, clap, open hat, tom, rim, crash — one
  piece per step per lane, two lanes.
* **The grid is straight 16ths** unless the song says `.swing(..)` /
  `.humanize(..)`; a 12-step lane against 16-step bars makes the grid
  breathe either way.
* **Voices**: raw shapes take the whole `Voice` (unison stack, filter
  envelope, vibrato, glide, sub, noise); the `Supersaw` / `DarkPad` /
  `DrivenBass` presets carry their own graph (envelope, ties, chords,
  sub, pan, drive and sends still apply).
* **The bus, per song**: `intensity` sets the level and the per-bar
  lowpass sweep's peak (higher = darker, tighter; `.sweep(..)` its
  depth); a section's `.ducked()` pumps the melodic lanes on every kick
  through the song's `Sidechain` — by default depth `DUCK_DEPTH` (0.65: a
  dip to 0.35) and a release of `DUCK_RECOVERY` (0.3 s) at the song's
  tempo.
* **Dynamics**: per channel per section (`.vel` → `Section::level`) times
  the per-step `.accents(..)`; no automation yet — a fade is a sequence of
  thinner sections or an accent ramp.
* **Bake budget**: each song's voice set (`music_keys`) must stay ≤ 96
  keys — distinct (degree, tied length, voicing) per melodic lane + the
  kit pieces used. The briefed tracks use 15–34, the v2 songs up to 53.

## The lane channels (what is NOT baked)

```text
 note ─► panner ─► drive ─► lowpass ─► level ─┬──────────────────────► ducker ─► bus ─► lowpass ─► soft-clip ─► out
                                             ├─ echo send ─► delay ─► tone ─► return ──┤
                                             │                 ▲           └─ feedback ─┘
                                             └─ verb send ─► convolver (hall) ─► return ─┘
 drum ────────────────────────────────────────────────────────────────► bus
```

A baked note is DRY: pan, drive, the lane's lowpass and level (what the
section ramps move — open / unity at rest), the echo / hall sends, the
duck and the per-bar sweep are live, per lane, re-pointed at every song
change (`AudioEngine::apply_voices`) and at every section start
(`schedule_section`). Two consequences worth knowing:

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
