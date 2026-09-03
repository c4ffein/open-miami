# DARKSYNTH — the house genre of Open Miami

The primary color of this game's soundtrack. Darksynth is synthwave's violent
sibling: where synthwave remembers the 80s fondly, darksynth remembers the
80s as a slasher film. It conveys PRESSURE — something mechanical, hostile
and rhythmically relentless. Perfect for a tower full of rogue machines.

## What it must convey
Menace with momentum. The listener should feel hunted AND propelled: the
track is on the killer's side. When a floor's darksynth track is right, the
player walks faster without noticing.

## Tempo
- 95–128 BPM. The heavier the track, the SLOWER it can afford to be —
  weight comes from the low end, not speed.
- Straight 4/4, 16th-note grid. Swing is off-genre here (that's wave).

## Harmony language
- Natural minor and PHRYGIAN. The phrygian b2 is the genre's signature
  menace note — use it in bass riffs (E–F–E movement) and pad voicings.
- Progressions stay narrow and modal: i–bII, i–bVI–bVII, or NO progression
  at all — a single-chord riff track is fully legitimate; movement then
  comes from riff variation and layer arrangement.
- Melodies are RIFFS, not songs: short (1–2 bar) cells, repeated with
  small mutations. A darksynth lead is a machine part, not a singer.
- Tension tools: chromatic approach notes, tritone stabs, half-step
  drops on the last beat of a phrase.

## Sound palette (engine mapping in brackets)
- BASS is the lead instrument: aggressive, mid-forward, driven/distorted
  saw or square, often playing 16th-note pump patterns [DRIVEN BASS
  preset — waveshaper soft clip; keep the fundamental strong].
- SUPERSAW stacks for stabs and the big refrain lead [SUPERSAW preset,
  3–5 voices detuned].
- Dark slow pads underneath, felt more than heard [DARKPAD preset,
  lowpassed, slow attack].
- Arps: 16ths, minor triads + b2 color, often gated rhythmically.
- The SIDECHAIN PUMP is non-negotiable: everything non-drum ducks on the
  kick [the duck curve — deep on heavy sections, subtle on intros].

## Drum grammar
- Kick: four-on-the-floor for drive sections; half-time (1 and 3, snare
  on 3) for the heavy/menace sections. Alternate between the two across
  sections — that alternation IS darksynth arrangement.
- Snare: big, on 2+4 (drive) or 3 (half-time). Occasional last-16th
  double as a fill.
- Hats: offbeat 8ths minimum; 16th runs for lift. Sparse = heavier.

## Structure (for 1:30–4:00 tracks)
- Cold open on the RIFF (bass alone or bass+kick) — 4–8 bars.
- Build by ADDING LAYERS, never by changing the riff: +hats, +arp,
  +pad, then the supersaw refrain.
- The REFRAIN is a function: same material each return, one degree
  hotter (extra octave, extra layer, deeper duck).
- Breakdown: strip to pad + one element, half-time feel, 4–8 bars,
  then slam the full kit back WITHOUT a long ramp — darksynth drops
  hit like a door.
- End cold or on a filtered loop — no fade-out fadeouts.

## Mix character
- Bass and kick own the track; lead sits behind the bass in energy.
- Grit everywhere is fine; mud is not — keep pads dark but quiet.
- Space: short dark reverb, slap delay on stabs. Dry punchy drums.

## Checklist before calling a darksynth track done
- [ ] Is the bass riff strong enough to carry 8 bars alone?
- [ ] Does the b2 (or an equivalent menace interval) appear?
- [ ] Does everything pump on the kick?
- [ ] Do drive and half-time sections alternate at least once?
- [ ] Does the refrain return at least twice, hotter each time?
- [ ] Would it still slap with the lead muted? (It must.)

## Engine reality (see `docs/MUSIC_CODE.md`)
- The pump is `.ducked()` per section; its depth and recovery are
  constants — "subtle on intros" means: don't duck the intro.
- Tritone stabs need a scale that contains the tritone (Locrian); in
  Phrygian the b2 stab is the menace interval.
- Stabs are short by nature (lead ≈ one 16th): write them as hits with air
  around them, not as held chords.
- No reverb / slap delay on the music bus: "space" is rests and velocity.
- Reference: `service_corridor.rs`, `thermal_mass.rs`.
