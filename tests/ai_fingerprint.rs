//! A REFACTORING TOOL, not a check: a behavioural fingerprint of the AI on
//! every floor (the autoplayer run + an idle-player run, 30 s each; every
//! enemy's position / heading / velocity / health / AI state and the world's
//! RNG, hashed every tick). Ignored by default — a frozen hash would fail on
//! every deliberate tuning change. To prove a refactor of `src/systems/ai/`
//! changes NOTHING, run it before and after and diff the output:
//!
//!   cargo test --test ai_fingerprint -- --ignored --nocapture | grep ^FP
//!
//! Mutation-tested when it was written: a 1 px change of the wall padding, a
//! different confusion look count, a 1 px change of an arrival radius and a
//! swapped feral speed constant each change it (4 to 27 of the 30 rows).
use open_miami::components::{Enemy, Health, Position, Rotation, Velocity, AI};
use open_miami::sim::Simulation;

fn mix(h: &mut u64, v: u64) {
    *h ^= v;
    *h = h.wrapping_mul(0x100000001b3);
}

fn fingerprint(sim: &Simulation, h: &mut u64) {
    let mut es = sim.world.query::<Enemy>();
    es.sort_by_key(|e| e.0);
    for e in es {
        mix(h, e.0);
        if let Some(p) = sim.world.get_component::<Position>(e) {
            mix(h, p.x.to_bits() as u64);
            mix(h, p.y.to_bits() as u64);
        }
        if let Some(r) = sim.world.get_component::<Rotation>(e) {
            mix(h, r.angle.to_bits() as u64);
        }
        if let Some(v) = sim.world.get_component::<Velocity>(e) {
            mix(h, v.x.to_bits() as u64);
            mix(h, v.y.to_bits() as u64);
        }
        if let Some(hp) = sim.world.get_component::<Health>(e) {
            mix(h, hp.current as u64);
        }
        if let Some(ai) = sim.world.get_component::<AI>(e) {
            for b in format!("{:?}", ai).bytes() {
                mix(h, b as u64);
            }
        }
    }
    mix(h, sim.world.rng_state() as u64);
}

#[test]
#[ignore = "a refactoring tool: run before / after and diff (see the file header)"]
fn ai_fingerprint() {
    let dt = 1.0 / 60.0;
    for level in 0..15 {
        for bot in [true, false] {
            let mut sim = Simulation::new(level);
            let mut h = 0xcbf29ce484222325u64;
            for _ in 0..1800 {
                if bot {
                    sim.bot_step(dt);
                } else {
                    sim.step(dt);
                }
                fingerprint(&sim, &mut h);
            }
            println!(
                "FP level={level} bot={bot} {h:016x} alive={}",
                sim.enemies_alive()
            );
        }
    }
}
