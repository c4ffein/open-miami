# WITCH HOUSE — the corruption frequency

The occult corner of the palette, reserved for the SHOGGOTH arc. Witch
house conveys WRONGNESS: ritual dread, signals that shouldn't exist,
something ancient wearing a machine's face. In Open Miami it bleeds into
the late floors as the CORRUPTOR's presence grows, and owns floor 13½.

## What it must convey
The music itself sounds infected. Familiar elements (a beat, a chord)
presented damaged — slowed, detuned, buried — so the player senses the
rules of the tower no longer apply.

## Tempo
- SLOW: 55–75 BPM felt, dragging deliberately. Nothing rushes.
- Grid intact but heavy — like darksynth played back at the wrong speed.

## Harmony language
- Minor with TRITONE emphasis and unresolved b5 colors; drones over a
  pedal that REFUSES cadence.
- Detuning as dread: parallel voices 20–40 cents apart, octaves that
  beat against each other [two voices, slight pitch offset].
- Melodic material: tiny chant-like cells (2–4 notes), repeated well past
  comfort, occasionally transposed by a tritone instead of a logical step.
- Silence and drone stretches are structural, not gaps.

## Sound palette (engine mapping)
- Deep, slow bass drones [DRIVEN BASS at low drive, long notes, or
  DARKPAD an octave down].
- Bent choir-ish pads: detuned pair, lowpassed, slow LFO wobble if
  available [DARKPAD + detune].
- A "damaged lead": a simple voice pitched down where you'd expect up,
  or playing the cell a tritone off.
- Percussive color: sparse metallic ticks/clicks (short filtered noise)
  instead of hats.
- Occasional darksynth artifacts allowed (a distant pump, a driven stab)
  — the corruption is eating THAT music.

## Drum grammar
- Trap-adjacent skeleton at funeral pace: kick sparse and heavy, big
  slow snare/clap on 3, hi-hat stutters in short bursts (fast 32nd
  clusters) that then vanish for bars.
- Rhythm should occasionally LIMP: drop an expected kick once per
  section — the missing heartbeat is the effect.

## Structure
- Ritual, not song: sections are STATES (drone → procession → seizure →
  drone), cycled with worsening corruption each pass.
- Transitions by decay: a layer detunes further, the lowpass closes, the
  beat loses a limb — rather than clean adds/cuts.
- 2:00–5:00; this genre can sustain length because it's about duration.

## Mix character
- Muffled and cavernous: long dark reverb, heavy lowpass on nearly
  everything; the sub is the clearest element in the room.
- Contrast tool: ONE bright element, rare, alarming when it appears.

## Checklist
- [ ] A drone/pedal that never cadences?
- [ ] Audible detune beating somewhere?
- [ ] A tritone move where a normal one was expected?
- [ ] A limping beat (missing expected hit) at least once per section?
- [ ] Does it feel like the *soundtrack itself* is corrupted?

## Engine reality (see `docs/MUSIC_CODE.md`)
- No detune / LFO / lowpass-closing per section: "decay" is arrangement —
  `sparsify` the cell, drop the kick, lower the velocity.
- Detune beating is approximated by the b2 held against the root; the
  tritone move is `transpose(4)` in Locrian.
- Metallic ticks are sparse hats; 32nd clusters are `hhhh` 16th bursts;
  the slow snare / clap is the snare.
- Tempo cannot change mid-song: "played back at the wrong speed" is
  `stretch(2)` on the lead.
- Reference: `crown_of_static.rs`, `signal_rot.rs`.
