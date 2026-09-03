# SYNTHWAVE — the nostalgia channel

The warm half of our palette. Synthwave conveys NEON NOSTALGIA: night
drives, sodium lamps, a city that looks better through a windshield. In
Open Miami it belongs to the frame around the violence — the title screen,
the drive backdrop, the ending ride, breather floors — not the fights.

## What it must convey
Longing with a pulse. Where darksynth pushes, synthwave GLIDES. The player
should feel the game exhale.

## Tempo
- 80–118 BPM. The classic cruise sits ~100.
- Straight 4/4; gentle 16th arps are the motion.

## Harmony language
- Natural minor, but WARM: lean on bVI and bVII major chords (the
  "sunset chords") — i–bVI–III–bVII is the genre's home progression.
- Add9 and sus2 colors on pads; avoid crunchy dissonance.
- Melodies are actual MELODIES here: 2–4 bar singable phrases, long
  notes allowed, call-and-answer between lead and arp.
- Harmonic rhythm: one chord per bar, unhurried.

## Sound palette (engine mapping)
- Pads FIRST: wide, slow-attack, brighter than darksynth's [DARKPAD
  preset with the lowpass opened up].
- Bass: round and supportive — clean square/saw, OFFBEAT 8th pump
  (the classic gallop: rest-note rest-note) [plain saw voice, light or
  no drive].
- Lead: expressive saw or triangle, portamento feel if possible; sits
  ON TOP of the mix (opposite of darksynth).
- Arps: constant gentle 16ths, the engine of the genre.
- Sidechain pump present but GENTLE — a breath, not a slam.

## Drum grammar
- Kick four-on-floor, softer; big gated-style snare on 2+4.
- Hats: offbeat 8ths, occasional open hat pushes.
- Fills: snare rolls into section changes — synthwave telegraphs its
  transitions (darksynth doesn't).

## Structure
- Intro: pad + arp establishing the chord loop (8 bars).
- Verse: + bass + drums. Chorus/refrain: + lead melody.
- The BRIDGE matters here: shift to bVI as a temporary home for 4–8
  bars, then return — that lift is the genre's emotional payoff.
- Outro may loop and filter down slowly (the one place a fade is ok).

## Mix character
- Everything washed: long reverb on snare and lead, chorus-y width on
  pads. Softer transients than darksynth throughout.

## Checklist
- [ ] Home progression uses bVI/bVII warmth?
- [ ] Offbeat bass pump present?
- [ ] A singable lead phrase that returns?
- [ ] A bridge (or equivalent lift) before the last refrain?
- [ ] Does it glide — could you drive to it?

## Engine reality (see `docs/MUSIC_CODE.md`)
- No portamento, no chorus, no long reverb: width comes from the
  `DarkPad` preset's detuned pair and from retriggering the pad every beat.
- "Long notes allowed" — not in this engine: a lead note is a 16th. Write
  the melody in 8ths/quarters with rests where the sustain would be.
- The gated snare is the plain snare; the fill is `ss` / `ssss`.
- The fade at the outro is a thinner section at a lower `.vel()`.
- Reference: `neon_checksum.rs`.
