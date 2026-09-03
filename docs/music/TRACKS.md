# TRACKS — the soundtrack

Per-track briefs for the code-composed soundtrack (see `docs/MUSIC_CODE.md`
for the authoring API and its engine limits, and the genre guides in this
directory). Every track states: what it conveys, its genre mix, target
length, structural notes — and where it lives. The genre-doc checklist is
written into each track's Rust file header, ticked.

Percentages are creative dials, not math — "70% darksynth / 30% wave"
means: darksynth grammar and drums, wave's harmony lushness or space
leaking in.

Roles are code: `title_song()`, `song_for_floor(floor_id)` and
`ending_song()` in `src/audio/songs.rs`, pinned by
`soundtrack_roles_follow_the_briefs`; the lengths by
`tracks_run_the_briefed_length`.

| # | Track | Role | File | Key / BPM | Length |
|---|-------|------|------|-----------|--------|
| 1 | NEON CHECKSUM | title screen | `neon_checksum.rs` | A minor, 100 | 2:34 |
| 2 | WALK DON'T RUN | floor 0 | `walk_dont_run.rs` | G minor, 88 | 2:00 |
| 3 | SERVICE CORRIDOR | floors 1–4 | `service_corridor.rs` | E phrygian, 118 | 2:59 |
| 4 | THERMAL MASS | floors 5–8 | `thermal_mass.rs` | B phrygian, 100 | 3:31 |
| 5 | SIGNAL ROT | floors 9–12 | `signal_rot.rs` | G locrian, 112 | 3:26 |
| 6 | CROWN OF STATIC | floors 13 + 13½ | `crown_of_static.rs` | F locrian, 70 | 4:34 |
| 7 | COAST HOME | the ending ride | `coast_home.rs` | A minor, 88 | 2:33 |

Status: all seven are COMPOSED and wired, none has been AUDITIONED yet —
see "Working method" below.

---

## 1. NEON CHECKSUM — title screen
- **Conveys**: the promise of the night. You're in the car, the tower
  glows ahead, nothing has gone wrong yet.
- **Mix**: 80% synthwave / 20% darksynth.
- **Length**: ~2:30 loop.
- **Notes**: home progression with bVI warmth (Am F C G), gentle pump
  (the duck armed under the refrains only), a lead that only fully
  arrives on the second refrain. Sits UNDER the engine idle — the bass
  pump is voiced an octave up so the lowest octave stays empty for the
  motor. Ends stripped to pad + arp, loops clean.
- **Built**: intro → verse → refrain(Cool: the call without the answer)
  → verse → bridge (bVI as the temporary home) → refrain(Warm) →
  refrain(Hot: the motif doubled an octave up, 16th hats) → outro.

## 2. WALK DON'T RUN — floor 0, the cold open
- **Conveys**: forced calm. Act normal past the gate; your hands know
  what's coming even if the guards don't.
- **Mix**: 50% wave / 40% synthwave / 10% darksynth.
- **Length**: ~2:00.
- **Notes**: half-time, an m9 bed (Gm with the 9th and 11th as sine
  raindrops phasing 3-against-4), the synthwave offbeat gallop on a
  triangle sub; a darksynth bass pattern appears LOW and quiet in the
  last full section (`undertow`, velocity 0.5) — the violence idling
  under the politeness.
- **Built**: drift → walk → gaze (the sigh; the bed lifts to Eb) → walk
  → undertow → crowd (thinner than the drift it came from).

## 3. SERVICE CORRIDOR — floors 1–4
- **Conveys**: the grind begins. Confident, athletic hostility — early
  floors where the player feels strong.
- **Mix**: 85% darksynth / 15% synthwave.
- **Length**: ~3:00.
- **Notes**: 118 BPM drive, four-on-floor, the bass riff IS the hook
  (root 16ths, octave, the b2 rub); synthwave leaks in as one warm bVI
  lift in the bridge, then back to the grind. Refrain returns three
  times, hotter each pass.
- **Built**: open (riff alone, then the kick) → build → refrain(Cool) →
  drive (the refrain with the lead muted) → half-time → refrain(Warm) →
  bridge → breakdown (4 bars, snare roll) → drive → refrain(Hot) →
  half-time → outro (cold).

## 4. THERMAL MASS — floors 5–8
- **Conveys**: the tower pushing back. Heavier, slower, meaner — the
  fights stop being free.
- **Mix**: 95% darksynth / 5% witch house.
- **Length**: ~3:30.
- **Notes**: 100 BPM, half-time sections dominate, a two-bar 8th riff
  with the b2 and the bVII below the root, every kicked section ducked.
  The witch-house 5% = `drone`: the pad refusing to move while the lead
  buzzes the b9 at a whisper — the first hint of the CORRUPTOR.
- **Built**: procession → slab → crush(Cool) → drive → crush(Warm) →
  drone → slab → crush(Hot) → drive → crush(Hot) → procession (cold).

## 5. SIGNAL ROT — floors 9–12
- **Conveys**: something is wrong with the building. Combat energy
  intact but the track keeps glitching toward ritual.
- **Mix**: 55% darksynth / 35% witch house / 10% wave.
- **Length**: ~3:30.
- **Notes**: darksynth `drive` sections alternate with `rot` — the same
  riff tritone-shifted (`transpose(4)` in Locrian), the lead at half
  speed, the pad parked on the tritone chord, the kick limping — and
  each cycle the corrupted variant lasts longer (4, 8, 12, 20 bars).
  The "refrain, hotter" pattern inverted: the refrain returns *sicker*.
  `haze` is the wave 10%.
- **Built**: intro → drive 8 → rot 4 → drive 8 → rot 8 → haze → drive 8
  → rot 12 → drive 4 → rot 20 → outro (the rot with the beat gone).

## 6. CROWN OF STATIC — floor 13 + 13½ (the boss)
- **Conveys**: the mask comes off. Ritual dread, then a slow crushing
  procession once the fight starts.
- **Mix**: 75% witch house / 25% darksynth.
- **Length**: ~4:30.
- **Notes**: opens as pure witch house (drone, chant cell with every
  fourth bar a tritone off, metallic ticks); the darksynth kit enters at
  funeral tempo — driven bass 8ths + full duck at 70 BPM — when the mask
  cracks (`procession(Warm)`). The one bright element: a single high
  supersaw stab, rare, on the mask-crack bars. The heartbeat goes
  missing on every fourth bar; `seizure` has no kick at all.
- **Built**: drone → chant → procession(Cool) → seizure → drone →
  procession(Warm) → seizure → procession(Hot) → chant(Hot) →
  procession(Hot) → drone.

## 7. COAST HOME — the ending ride
- **Conveys**: it's over. Grief and relief in the same breath; the
  warp trails carry you out.
- **Mix**: 60% wave / 40% synthwave.
- **Length**: ~2:30.
- **Notes**: half-time, an m11 bed (Am with the raindrops' 9th and 11th),
  the title track's lead motif quoted ONCE, slower and lower
  (`stretch(2)`, `transpose(-7)` of `neon_checksum::motif()`) — the
  night remembering itself. Ends emptier than it starts, into the
  credits' silence. The calmest track in the list (the ending pick).
- **Built**: emerge → swell → remember → drift → swell → thin → gone.

---

## Working method
1. Compose against the brief + genre checklist, in Rust (one file per
   track, refrains as functions), knowing the engine's limits
   (`docs/MUSIC_CODE.md`, "What the engine can and cannot do").
2. The author cannot hear: structure and theory get it 80% there; the
   user auditions in the `?viz` TRACKER (click a track, click a section
   miniature to jump) and reports per section ("groove too thin", "duck
   too deep") — iterate function by function.
3. The floor mapping is live from the first commit (there is no separate
   audition list any more); a track that fails audition is fixed in
   place, not swapped.
