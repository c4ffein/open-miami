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

use super::webaudio::{graphs, reset_graphs, EventKind, Graph, NodeKind};
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
            let g = offline_graph(|| {
                engine.render_music_slot(i);
            });
            let g = g.borrow();
            let len = g.offline_frames.unwrap() as f64 / g.sample_rate as f64;
            check_voice(&format!("song {si} {key:?}"), &g, 0.0, Some(len));
        }
    }
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
