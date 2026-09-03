# WAVE — the dissociative undertow

The introspective corner of the palette. Wave (the SoundCloud-era sense:
hazy, half-tempo, melancholic electronic) conveys DISSOCIATION — beauty
heard through glass, grief at 2 AM. In Open Miami it colors the moments
BETWEEN violence: dialogue beats, aftermath floors, the seconds after a
purge when the counter hits zero and the room is suddenly quiet.

## What it must convey
Floating detachment. The player just did something terrible and the music
declines to comment. Sad, weightless, a little beautiful.

## Tempo
- 60–90 BPM felt tempo, and HALF-TIME grooves above that (a 140 grid
  with drums moving at 70 is idiomatic).
- Swing/shuffle is welcome here — the only genre in our palette where
  the grid should breathe (approximate with off-grid 16th placement).

## Harmony language
- Minor with EXTENSIONS: m9, m11, add9 — lush, unresolved voicings.
- Slow harmonic rhythm: 2–4 bars per chord, sometimes one chord for a
  whole section over a moving bass.
- Melodies fragmentary and repeating — a 3-note sigh, not a phrase.
  Space between notes matters more than the notes.
- Detune/pitch-drift as an expressive device: slightly bent unisons,
  the "warped tape" feel [two voices a few cents apart].

## Sound palette (engine mapping)
- Sub-heavy sine/triangle bass, long notes, sliding when possible.
- Bell-ish or glassy plucks (triangle/sine with fast decay) drowned in
  reverb — the raindrop layer [sine voice + the reverb bus, loud send].
- Pads: the darkest, slowest, most filtered [DARKPAD, lowpass nearly
  closed, attack in seconds].
- Almost no supersaw, almost no drive — this genre is CLEAN and cold.

## Drum grammar
- Sparse half-time: kick on 1 (and maybe the "and" of 3), snare/clap
  on 3. Whole bars of no drums are idiomatic.
- Hats: lazy, quiet, sometimes triplet ghosts; or none.
- No fills. Sections change by texture, not drum announcements.

## Structure
- Loop-based and additive/subtractive: a 4–8 bar core loop; sections
  differ by which layers are present.
- Arc: emerge from near-silence → thicken → strip back below where it
  started. End emptier than the beginning.
- 1:30–3:00 is plenty; wave overstays quickly.

## Mix character
- Reverb is an instrument: long, dark, everything distant except the
  bass, which stays close and dry.
- Dynamics low and flat — no drops, no slams.

## Checklist
- [ ] Half-time feel with real emptiness between hits?
- [ ] At least one lush extended chord (m9/m11) sustained long?
- [ ] A repeating fragment that never fully resolves?
- [ ] Bass dry and close while everything else floats far away?
- [ ] Does it feel like aftermath?

## Engine reality (see `docs/MUSIC_CODE.md`)
- No swing, no triplets, no off-grid placement: the grid breathes through
  a 12-step lane looping inside 16-step bars (3-against-4).
- No slides, no detuned unisons, no reverb: the "raindrop" layer is the
  sine arp at a low velocity with lots of rests; "distance" is velocity.
- m9 / m11: the pad blooms a triad only — put the 9th / 11th in the arp or
  lead over it.
- Sub bass "long notes" are thumps (≈ two 16ths): place them sparsely.
- Reference: `walk_dont_run.rs`, `coast_home.rs`.
