//! Host tests of the WebAudio engine, through the recording mock of
//! `webaudio.rs`: the REAL voice builders run natively and the node graphs
//! they build are read back. What is asserted are the invariants a synth
//! recipe can silently break — silently, because the engine swallows every
//! Web Audio error by design (it must never take the game down):
//!
//! - an exponential ramp to a value <= 0, a negative time or a zero-length
//!   curve THROWS in Web Audio: the envelope is then simply missing;
//! - an oscillator that is never stopped runs (and costs) forever;
//! - a node that reaches no destination is built, scheduled and inaudible;
//! - an event scheduled before `currentTime` is dropped or smeared;
//! - a sound longer than its bake length is TRUNCATED once pre-rendered —
//!   the baked and the live version of the same sound then differ.

use super::webaudio::{graphs, reset_graphs, EventKind, Graph, NodeKind, MOCK_SAMPLE_RATE};
use super::*;

/// The graph the offline bake of `build` assembled (the last offline context
/// created while it ran).
fn offline_graph(build: impl FnOnce()) -> std::rc::Rc<std::cell::RefCell<Graph>> {
    let before = graphs().len();
    build();
    graphs()
        .into_iter()
        .skip(before)
        .rfind(|g| g.borrow().offline_frames.is_some())
        .expect("the bake created an OfflineAudioContext")
}

/// The invariants of ONE voice graph. `len` = the seconds it may occupy
/// (`None`: unbounded, a live bus).
fn check_voice(name: &str, g: &Graph, earliest: f64, len: Option<f64>) {
    let sources: Vec<usize> = (0..g.nodes.len())
        .filter(|&i| {
            matches!(
                g.nodes[i].kind,
                NodeKind::Oscillator | NodeKind::BufferSource
            )
        })
        .collect();
    assert!(!sources.is_empty(), "{name}: builds no sound source at all");
    for &i in &sources {
        let n = &g.nodes[i];
        let (when, offset) = n
            .start
            .unwrap_or_else(|| panic!("{name}: source #{i} is never started"));
        assert!(
            when.is_finite() && when >= earliest,
            "{name}: source #{i} starts at {when} (< {earliest})"
        );
        assert!(
            offset.is_finite() && offset >= 0.0,
            "{name}: source #{i} offset {offset}"
        );
        if n.kind == NodeKind::Oscillator {
            let stop = n
                .stop
                .unwrap_or_else(|| panic!("{name}: oscillator #{i} is never stopped"));
            assert!(
                stop > when,
                "{name}: oscillator #{i} stops ({stop}) before it starts ({when})"
            );
        }
        if n.kind == NodeKind::BufferSource {
            assert!(
                n.buffer_frames.is_some(),
                "{name}: buffer source #{i} has no buffer"
            );
        }
    }
    for i in 1..g.nodes.len() {
        let drives_a_param = g.mod_edges.iter().any(|m| m.0 == i || g.reaches(i, m.0));
        assert!(
            g.reaches(i, 0) || drives_a_param,
            "{name}: node #{i} ({:?}) reaches no destination",
            g.nodes[i].kind
        );
    }
    for e in &g.events {
        assert!(
            e.value.is_finite(),
            "{name}: {:?} {} = {} on node #{}",
            e.kind,
            e.param,
            e.value,
            e.node
        );
        assert!(
            e.time.is_finite() && e.time >= earliest,
            "{name}: {:?} {} at t = {} (< {earliest})",
            e.kind,
            e.param,
            e.time
        );
        match e.kind {
            EventKind::ExpRamp => assert!(
                e.value > 0.0,
                "{name}: exponential ramp of {} to {} (throws: must be > 0)",
                e.param,
                e.value
            ),
            EventKind::Curve => assert!(
                e.end > e.time,
                "{name}: zero-length value curve on {}",
                e.param
            ),
            _ => {}
        }
    }
    if let Some(len) = len {
        let last = g
            .events
            .iter()
            .map(|e| e.end.max(e.time))
            .chain(g.nodes.iter().filter_map(|n| n.stop))
            .fold(0.0, f64::max);
        assert!(last <= len + 1e-6, "{name}: schedules up to t = {last:.3} s but is baked for {len:.3} s (the bake truncates it)");
    }
}

#[test]
fn the_live_buses_all_reach_the_destination() {
    reset_graphs();
    let engine = AudioEngine::new();
    assert!(engine.is_enabled());
    let live = &graphs()[0];
    let g = live.borrow();
    assert!(g.offline_frames.is_none());
    assert!(
        g.count(NodeKind::Convolver) >= 2,
        "the melee room + the gun / hit room"
    );
    assert!(g.count(NodeKind::Compressor) >= 1);
    for i in 1..g.nodes.len() {
        assert!(
            g.reaches(i, 0),
            "live bus node #{i} ({:?}) reaches no destination",
            g.nodes[i].kind
        );
    }
    for c in (0..g.nodes.len()).filter(|&i| g.nodes[i].kind == NodeKind::Convolver) {
        assert!(
            g.nodes[c].buffer_frames.is_some(),
            "convolver #{c} has no impulse response"
        );
    }
}

#[test]
fn every_sfx_bakes_a_sound_graph_that_fits_its_spec() {
    reset_graphs();
    let engine = AudioEngine::new();
    for kind in SFX_KINDS {
        let g = offline_graph(|| {
            engine.render_variant(kind);
        });
        let g = g.borrow();
        let frames = g.offline_frames.unwrap() as f64;
        let len = frames / g.sample_rate as f64;
        assert!(
            (len - kind.spec().len).abs() < 1e-3,
            "{kind:?}: baked for {len} s, spec says {}",
            kind.spec().len
        );
        check_voice(&format!("{kind:?}"), &g, 0.0, Some(len));
    }
}

#[test]
fn every_song_bakes_every_note_voice_and_each_fits_its_length() {
    for (si, song) in SONGS.iter().enumerate() {
        reset_graphs();
        let mut engine = AudioEngine::new();
        engine.set_song(*song);
        let keys = music_keys(song);
        assert!(!keys.is_empty(), "song {si} schedules no voice");
        for (i, key) in keys.iter().enumerate() {
            let computed =
                matches!(key, MusicKey::Note { lane, .. } if song.voices[*lane].wave.is_computed());
            if computed {
                // Rendered in Rust, no graph: the buffer lands at once.
                assert!(engine.render_music_slot(i), "song {si} {key:?}");
                let slots = engine.baked_music.slots.borrow();
                let frames = slots[i].buf.as_ref().map(|b| b.frames);
                let expect = (key_seconds(song, *key) * f64::from(MOCK_SAMPLE_RATE)).ceil() as u32;
                assert_eq!(frames, Some(expect), "song {si} {key:?}");
                continue;
            }
            let g = offline_graph(|| {
                engine.render_music_slot(i);
            });
            let g = g.borrow();
            let len = g.offline_frames.unwrap() as f64 / g.sample_rate as f64;
            check_voice(&format!("song {si} {key:?}"), &g, 0.0, Some(len));
        }
    }
}

/// The SETTINGS music level is the music bus's gain — clamped, applied at
/// once, and it leaves the SFX path alone.
#[test]
fn the_music_level_is_the_bus_gain() {
    reset_graphs();
    let engine = AudioEngine::new();
    let bus = engine.music_bus.as_ref().unwrap();
    assert_eq!((engine.music_level(), bus.gain().value()), (1.0, 1.0));
    let events = graphs()[0].borrow().events.len();
    engine.set_music_level(0.25);
    assert_eq!((engine.music_level(), bus.gain().value()), (0.25, 0.25));
    engine.set_music_level(7.0);
    assert_eq!(bus.gain().value(), 1.0);
    engine.set_music_level(f64::NAN);
    assert_eq!(bus.gain().value(), 1.0);
    engine.set_music_level(-1.0);
    assert_eq!(bus.gain().value(), 0.0);
    let g = graphs()[0].borrow().events[events..].to_vec();
    assert!(g
        .iter()
        .all(|e| e.node == bus.as_ref().id && e.param == "gain"));
}

/// A COMPUTED voice (the strings of `audio/dsp.rs`) bakes synchronously —
/// no offline context, nothing in flight — and its live sketch is one
/// plain oscillator per partial like any other voice's.
#[test]
fn computed_voices_bake_at_once_and_sketch_like_the_rest() {
    reset_graphs();
    let mut engine = AudioEngine::new();
    let song = song_named("Salt Road");
    engine.set_song(song);
    let contexts = graphs().len();
    let keys = music_keys(&song);
    let computed: Vec<usize> = (0..keys.len())
        .filter(|&i| matches!(keys[i], MusicKey::Note { lane, .. } if song.voices[lane].wave.is_computed()))
        .collect();
    assert!(computed.len() > 20, "{} computed keys", computed.len());
    for &i in &computed {
        assert!(engine.render_music_slot(i));
    }
    assert_eq!(graphs().len(), contexts, "a computed bake opened a context");
    assert_eq!(engine.renders_in_flight.get(), 0);
    assert!(computed
        .iter()
        .all(|&i| engine.baked_music.slots.borrow()[i].buf.is_some()));
    // A drum of the same song still goes through the offline render.
    let drum = keys
        .iter()
        .position(|k| matches!(k, MusicKey::Drum(_)))
        .unwrap();
    assert!(engine.computed_bake(keys[drum]).is_none());
    // The sketch of a guitar note: one triangle; of a violin: one saw.
    let live = std::rc::Rc::clone(&graphs()[0]);
    let mut osc_types = Vec::new();
    for lane in [ARP, LEAD] {
        let key = keys
            .iter()
            .find(
                |k| matches!(k, MusicKey::Note { lane: l, chord: Chord::Single, .. } if *l == lane),
            )
            .unwrap();
        let before = live.borrow().nodes.len();
        engine.synth_music_note(*key, 1.0, 1.0, false);
        let g = live.borrow();
        let oscs: Vec<String> = g.nodes[before..]
            .iter()
            .filter(|n| n.kind == NodeKind::Oscillator)
            .map(|n| n.type_name.clone().unwrap())
            .collect();
        osc_types.push(oscs);
    }
    assert_eq!(osc_types, [vec!["Triangle"], vec!["Sawtooth"]]);
}

/// The song named `name` (the tracker's list).
fn song_named(name: &str) -> SongSpec {
    *SONGS
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("{name} is not in SONGS"))
}

/// The id of a persistent node in the live graph.
fn id(node: &impl AsRef<webaudio::AudioNode>) -> usize {
    node.as_ref().id
}

/// The per-lane channels exist and are wired in order — panner → drive →
/// ducker → bus → lowpass → soft-clip → out, with the echo (a feedback loop)
/// and the hall tapped after the drive and returning into the ducker; the
/// drums' entry (the bus) is past the ducker.
#[test]
fn the_lane_channels_are_wired_in_order() {
    reset_graphs();
    let engine = AudioEngine::new();
    let live = std::rc::Rc::clone(&graphs()[0]);
    let g = live.borrow();
    assert_eq!(engine.music_pan.len(), NUM_VOICES);
    assert_eq!(engine.music_drive.len(), NUM_VOICES);
    assert_eq!(g.count(NodeKind::Panner), NUM_VOICES);
    assert_eq!(g.count(NodeKind::Delay), 1);
    assert!(
        g.count(NodeKind::Convolver) >= 3,
        "two SFX rooms + the hall"
    );
    let fx = engine
        .music_fx
        .as_ref()
        .expect("the echo + hall were built");
    let (duck, bus) = (
        id(engine.music_duck.as_ref().unwrap()),
        id(engine.music_bus.as_ref().unwrap()),
    );
    let filt = id(engine.music_filter.as_ref().unwrap());
    assert!(g.edges.contains(&(duck, bus)) && g.edges.contains(&(bus, filt)));
    // The lowpass leaves through the safety soft-clip (a static curve: no
    // compressor — and so no automatic make-up gain — on the music path).
    let next = |n: usize| -> Vec<usize> {
        let to = g.edges.iter().filter(|e| e.0 == n).map(|e| e.1);
        to.collect()
    };
    let pre = next(filt);
    assert_eq!(pre.len(), 1);
    let clip = next(pre[0]);
    assert_eq!(clip.len(), 1);
    assert_eq!(g.nodes[clip[0]].kind, NodeKind::WaveShaper);
    assert_eq!(next(clip[0]), vec![0]);
    let music_path = [duck, bus, filt, pre[0], clip[0]];
    assert!(music_path
        .iter()
        .all(|&n| g.nodes[n].kind != NodeKind::Compressor));
    let pre_gain = g.events.iter().find(|e| e.node == pre[0]).unwrap().value;
    let curve = AudioEngine::softclip_curve(0.7, 4096);
    // Unity under the knee: a 0.3 input leaves as 0.3.
    let at = |x: f32| curve[((x * pre_gain + 1.0) / 2.0 * 4095.0).round() as usize];
    assert!((at(0.3) - 0.3).abs() < 1e-3, "{}", at(0.3));
    assert!(!g.reaches(bus, duck), "the drums' entry is past the ducker");
    for lane in 0..NUM_VOICES {
        let (pan, drive) = (id(&engine.music_pan[lane]), id(&engine.music_drive[lane]));
        assert!(
            g.edges.contains(&(pan, drive)),
            "lane {lane}: panner → drive"
        );
        assert!(
            g.edges.contains(&(drive, duck)),
            "lane {lane}: drive → ducker"
        );
        let (echo, verb) = (id(&fx.echo_send[lane]), id(&fx.verb_send[lane]));
        assert!(g.edges.contains(&(drive, echo)) && g.edges.contains(&(drive, verb)));
        assert!(g.edges.contains(&(echo, id(&fx.delay))));
    }
    // The echo repeats: delay → tone → feedback → delay; and it returns.
    let (delay, tone, fb) = (id(&fx.delay), id(&fx.tone), id(&fx.feedback));
    assert!(g.edges.contains(&(delay, tone)) && g.edges.contains(&(tone, fb)));
    assert!(g.edges.contains(&(fb, delay)));
    assert!(g.reaches(tone, duck));
}

/// A song change re-points the lane channels: pans, sends, the echo line.
#[test]
fn a_song_change_applies_its_voices_to_the_lanes() {
    reset_graphs();
    let mut engine = AudioEngine::new();
    let song = song_named("Sodium Lights");
    engine.set_song(song);
    let fx = engine.music_fx.as_ref().unwrap();
    for lane in 0..NUM_VOICES {
        let v = song.voices[lane];
        assert_eq!(
            engine.music_pan[lane].pan().value(),
            v.pan as f32,
            "pan {lane}"
        );
        assert_eq!(
            fx.echo_send[lane].gain().value(),
            v.echo as f32,
            "echo {lane}"
        );
        assert_eq!(
            fx.verb_send[lane].gain().value(),
            v.reverb as f32,
            "hall {lane}"
        );
    }
    assert!(
        song.voices.iter().any(|v| v.pan != 0.0) && song.voices.iter().any(|v| v.reverb > 0.0),
        "the test song must exercise the lanes"
    );
    let secs = (song.echo.steps * step_dur(&song)) as f32;
    assert_eq!(fx.delay.delay_time().value(), secs);
    assert_eq!(fx.feedback.gain().value(), song.echo.feedback as f32);
    // Back to a plain built song: everything centred and dry again.
    engine.set_song(song_named("Walk Don't Run"));
    let fx = engine.music_fx.as_ref().unwrap();
    for lane in 0..NUM_VOICES {
        assert_eq!(engine.music_pan[lane].pan().value(), 0.0);
        assert_eq!(fx.verb_send[lane].gain().value(), 0.0);
    }
    // The drive curve is finite, odd and monotonic, whatever the drive.
    for drive in [0.05, 0.5, 1.0] {
        let c = AudioEngine::drive_curve(drive, 2048);
        assert!(c.iter().all(|v| v.is_finite()));
        assert!(
            c.windows(2).all(|w| w[0] <= w[1]),
            "drive {drive}: not monotonic"
        );
        assert!((c[0] + c[2047]).abs() < 1e-6, "drive {drive}: not odd");
    }
}

/// A WIDE unison voice bakes to a stereo buffer (its stack is spread inside
/// it), everything else to mono; a tied note HOLDS its peak for the tied
/// steps before the pluck.
#[test]
fn wide_voices_bake_stereo_and_ties_hold_the_peak() {
    reset_graphs();
    let mut engine = AudioEngine::new();
    let song = song_named("Sodium Lights");
    engine.set_song(song);
    let (mut wide, mut mono, mut held) = (0, 0, 0);
    for (i, key) in music_keys(&song).iter().enumerate() {
        let g = offline_graph(|| {
            engine.render_music_slot(i);
        });
        let g = g.borrow();
        let is_wide = matches!(key, MusicKey::Note { lane, .. } if song.voices[*lane].is_wide());
        assert_eq!(g.offline_channels, if is_wide { 2 } else { 1 }, "{key:?}");
        if is_wide {
            wide += 1;
            assert!(g.count(NodeKind::Panner) >= 2, "{key:?}: no spread");
        } else {
            mono += 1;
            assert_eq!(g.count(NodeKind::Panner), 0, "{key:?}: a mono bake pans");
        }
        if let MusicKey::Note { len, lane, .. } = *key {
            if len > 1 && !song.voices[lane].wave.is_preset() {
                held += 1;
                let (_, _, attack) = voice_shape(&song, lane);
                let at = attack + step_dur(&song) * f64::from(len - 1);
                let holds = g.events.iter().any(|e| {
                    e.param == "gain" && e.kind == EventKind::Set && (e.time - at).abs() < 1e-9
                });
                assert!(holds, "{key:?}: no peak hold at {at:.3} s");
            }
        }
    }
    assert!(wide > 0 && mono > 0 && held > 0, "{wide} {mono} {held}");
}

/// The bake-time level of a melodic note: a `compose`-built song makes the
/// lane panners' centre law up (√2 — a plain centred lane is exactly as
/// loud as before the lane graph existed), a `const`-literal song mixed
/// with the panners in place does not.
#[test]
fn built_songs_make_up_the_centre_pan_law() {
    for (name, make_up) in [
        ("Walk Don't Run", std::f64::consts::SQRT_2),
        ("Neon Lounge", 1.0),
    ] {
        reset_graphs();
        let mut engine = AudioEngine::new();
        let song = song_named(name);
        assert_eq!(song.melodic_gain, make_up, "{name}");
        engine.set_song(song);
        let keys = music_keys(&song);
        let i = keys
            .iter()
            .position(|k| {
                matches!(
                    k,
                    MusicKey::Note {
                        lane: BASS,
                        chord: Chord::Single,
                        ..
                    }
                ) && song.voices[BASS].oscillators() == 1
                    && !song.voices[BASS].wave.is_preset()
            })
            .unwrap_or_else(|| panic!("{name}: no plain bass note"));
        let g = offline_graph(|| {
            engine.render_music_slot(i);
        });
        let g = g.borrow();
        let peak = g
            .events
            .iter()
            .filter(|e| e.param == "gain" && e.kind == EventKind::ExpRamp)
            .map(|e| e.value)
            .fold(0.0, f32::max);
        let expect = (MUSIC_GAIN * song.intensity * lane_shape(BASS).1 * make_up) as f32;
        assert!(
            (peak - expect).abs() < 1e-6,
            "{name}: peak {peak}, expected {expect}"
        );
    }
}

/// Nothing is baked natively, so every scheduled note takes the LIVE path —
/// the sketch: ahead of the clock, and bounded (one plain oscillator per
/// partial: no stack, no per-note filter, no panner) however rich the voice.
#[test]
fn the_live_sketch_is_bounded_and_ahead_of_the_clock() {
    for song in SONGS.iter() {
        reset_graphs();
        let mut engine = AudioEngine::new();
        engine.set_song(*song);
        let live = std::rc::Rc::clone(&graphs()[0]);
        live.borrow_mut().now = 7.25;
        for key in music_keys(song) {
            let (nodes, events) = (live.borrow().nodes.len(), live.borrow().events.len());
            engine.music_note(key, 7.30, 0.5);
            let g = live.borrow();
            let built = &g.nodes[nodes..];
            if let MusicKey::Note { chord, lane, .. } = key {
                let partials = chord.degrees().len();
                let oscs = built
                    .iter()
                    .filter(|n| n.kind == NodeKind::Oscillator)
                    .count();
                let noise = song.voices[lane].wave == Wave::Noise;
                assert_eq!(
                    oscs,
                    if noise { 0 } else { partials },
                    "{} {key:?}",
                    song.name
                );
                assert!(
                    built.len() <= 2 * partials,
                    "{} {key:?}: {} nodes",
                    song.name,
                    built.len()
                );
                assert!(built.iter().all(|n| !matches!(
                    n.kind,
                    NodeKind::Biquad | NodeKind::Panner | NodeKind::WaveShaper
                )));
            }
            for n in built {
                if let Some((when, _)) = n.start {
                    assert!(when >= 7.30, "{} {key:?}: starts at {when}", song.name);
                }
            }
            for e in &g.events[events..] {
                assert!(
                    e.time >= 7.30 && e.value.is_finite(),
                    "{} {key:?}: {e:?}",
                    song.name
                );
            }
            for i in nodes..g.nodes.len() {
                assert!(
                    g.reaches(i, 0),
                    "{} {key:?}: node #{i} is orphaned",
                    song.name
                );
            }
        }
    }
}

/// The scheduler, driven against the mock clock: every song schedules
/// notes, never in the past (humanize included); kicks pump the ducker —
/// down in 4 ms, an exponential release — exactly in the sections that
/// duck, and never in a song without a ducked section.
#[test]
fn the_scheduler_is_ahead_of_the_clock_and_ducks_by_section() {
    for song in SONGS.iter() {
        reset_graphs();
        let mut engine = AudioEngine::new();
        engine.set_song(*song);
        let live = std::rc::Rc::clone(&graphs()[0]);
        let duck = id(engine.music_duck.as_ref().unwrap());
        let (nodes, events) = (live.borrow().nodes.len(), live.borrow().events.len());
        live.borrow_mut().now = 3.0;
        engine.start_music();
        let total: usize = song.sections.iter().map(section_len).sum();
        let (mut now, mut kicks) = (3.0, 0);
        // One play-through, a frame (16 ms) at a time.
        let frames = (total as f64 * step_dur(song) / 0.016) as usize + 16;
        let mut ph = engine.playhead;
        for _ in 0..frames {
            engine.update(0.0);
            while ph != engine.playhead {
                let sec = &song.sections[ph.section];
                kicks += usize::from(sec.duck && is_kick_step(sec, ph.step));
                ph.advance(song);
            }
            now += 0.016;
            live.borrow_mut().now = now;
        }
        let g = live.borrow();
        let started: Vec<f64> = g.nodes[nodes..]
            .iter()
            .filter_map(|n| n.start.map(|s| s.0))
            .collect();
        assert!(
            started.len() > 50,
            "{}: {} sources in a play-through",
            song.name,
            started.len()
        );
        assert!(
            started.iter().all(|&t| t >= 3.0),
            "{}: a source in the past",
            song.name
        );
        let mut targets = 0;
        for e in &g.events[events..] {
            let is_static = e.kind == EventKind::Set && e.time == 0.0;
            assert!(
                is_static || e.time >= 3.0,
                "{}: {e:?} in the past",
                song.name
            );
            assert!(e.value.is_finite(), "{}: {e:?}", song.name);
            if e.node == duck {
                let depth = song.sidechain.depth as f32;
                match e.kind {
                    EventKind::Target => {
                        targets += 1;
                        assert_eq!(e.value, 1.0);
                    }
                    EventKind::LinearRamp => assert!((e.value - (1.0 - depth)).abs() < 1e-6),
                    EventKind::Set => assert!((1.0 - depth - 1e-6..=1.0).contains(&e.value)),
                    _ => panic!("{}: {e:?} on the ducker", song.name),
                }
            }
        }
        let expect = if song.sidechain.active() { kicks } else { 0 };
        assert_eq!(
            targets, expect,
            "{}: ducks vs kicks in ducked sections",
            song.name
        );
    }
    assert!(!song_named("Walk Don't Run").sections.iter().any(|s| s.duck));
    // Humanize moves a note both ways, but never into the clock's past.
    reset_graphs();
    let mut engine = AudioEngine::new();
    let loose = *SONGS
        .iter()
        .find(|s| s.humanize > 0.0)
        .expect("a humanized song");
    engine.set_song(loose);
    graphs()[0].borrow_mut().now = 5.0;
    let at: Vec<f64> = (0..200).map(|_| engine.humanized(5.001)).collect();
    assert!(at
        .iter()
        .all(|&t| t >= 5.0 && t <= 5.001 + loose.humanize + 1e-12));
    assert!(at.iter().any(|&t| t > 5.001) && at.iter().any(|&t| t < 5.001));
}

/// Live one-shots (nothing is baked natively) are scheduled AHEAD of the audio
/// clock — never in the past — and build the same kind of graph as the bake.
#[test]
fn live_one_shots_are_scheduled_ahead_of_the_clock() {
    reset_graphs();
    let engine = AudioEngine::new();
    let live = std::rc::Rc::clone(&graphs()[0]);
    live.borrow_mut().now = 7.25;
    let (nodes, events) = (live.borrow().nodes.len(), live.borrow().events.len());
    engine.play_attack_gun();
    engine.play_hit_club();
    engine.play_pickup();
    engine.play_player_hurt();
    let g = live.borrow();
    assert!(g.nodes.len() > nodes, "the live path built nothing");
    for e in &g.events[events..] {
        // `set_value_at_time(v, 0.0)` is the engine's idiom for a STATIC value
        // (valid: an event in the past applies at once); anything that is
        // part of an envelope must lie ahead of the clock.
        let static_value = e.kind == EventKind::Set && e.time == 0.0;
        assert!(
            static_value || e.time >= 7.25,
            "{:?} {} scheduled at {} — in the past of t = 7.25",
            e.kind,
            e.param,
            e.time
        );
    }
    for n in &g.nodes[nodes..] {
        if let Some((when, _)) = n.start {
            // SFX_LEAD is the nominal head start; a layer may lead its voice
            // by a hair (a pre-transient) — what must hold is "not in the past".
            assert!(
                when > 7.25,
                "a source starts at {when}: not ahead of the clock (t = 7.25)"
            );
            assert!(
                when < 7.25 + SFX_LEAD + 1.0,
                "a source starts at {when}: seconds late"
            );
        }
    }
}

/// The engine's jitter RNG is seeded: the same calls build the same graphs.
#[test]
fn the_engine_is_deterministic() {
    let run = || {
        reset_graphs();
        let engine = AudioEngine::new();
        engine.play_attack_shotgun();
        engine.play_hit_gun();
        engine.render_variant(SfxKind::EnemyDown);
        let mut engine = engine;
        engine.set_song(song_named("Blood Engine"));
        engine.render_music_slot(0);
        engine.start_music();
        engine.update(0.0);
        graphs()
            .iter()
            .map(|g| {
                let g = g.borrow();
                (g.nodes.clone(), g.edges.clone(), g.events.clone())
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}

/// The seam only holds if the engine never reaches around it: every Web Audio
/// type is named through `webaudio::`, so that the native build (and these
/// tests) see every call. `web_sys` may appear in comments and in webaudio.rs.
#[test]
fn the_engine_names_web_audio_only_through_the_seam() {
    for (name, src) in [
        ("engine.rs", include_str!("../engine.rs")),
        ("engine/bake.rs", include_str!("bake.rs")),
        ("engine/bus.rs", include_str!("bus.rs")),
        ("engine/music.rs", include_str!("music.rs")),
        ("engine/sfx_play.rs", include_str!("sfx_play.rs")),
        ("engine/voices.rs", include_str!("voices.rs")),
    ] {
        for (i, line) in src.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            assert!(
                !code.contains("web_sys"),
                "src/audio/{name}:{}: `web_sys` named directly — go through `webaudio`",
                i + 1
            );
        }
    }
}
