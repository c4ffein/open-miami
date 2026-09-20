//! The runtime that plays a floor: triggers that fire once, timers, tutorial
//! gates, conversations, holds, camera looks, checkpoints' scenario half.

use std::collections::VecDeque;

use super::comms::*;
use super::defs::*;
use super::world::*;
use crate::components::{Elevator, Player, Position};
use crate::ecs::World;
use crate::math::Vec2;
use crate::systems::elevator::ElevatorSystem;

/// The per-tick world facts a trigger is evaluated against.
struct TriggerCtx {
    player_pos: Option<Vec2>,
    kills: usize,
    alive: usize,
    boss_dead: bool,
    extracted: bool,
}

/// Live state of a floor's scenario.
#[derive(Debug, Clone)]
pub struct ScenarioState {
    floor: &'static FloorDef,
    /// Seconds since the floor started.
    time: f32,
    /// Per step: the time it fired (`None` = not yet).
    fired_at: Vec<Option<f32>>,
    /// Legacy rule active: no step opens an exit, so all-dead opens them all.
    auto_open_on_all_dead: bool,
    auto_opened: bool,
    /// Ids of exits opened so far (in order), for `exit_open` triggers.
    opened_exits: Vec<&'static str>,
    pub objective: String,
    pub comms: CommsFeed,
    /// One-shot sound effects requested since the last drain.
    sfx: Vec<&'static str>,
    /// The running `hold` (input lock), if any: see [`ScenarioState::hold`].
    hold: Option<HoldState>,
    /// The running `look_at`, if any: see [`ScenarioState::look_at`].
    look: Option<LookState>,
    /// The running `talk` conversation (dialogue mode), if any.
    dialogue: Option<DialogueState>,
    /// Per step: the time its `talk` conversation finished (`None` = not yet
    /// or the step has no `talk`). A `timer { after }` on a talking step
    /// counts from this instead of the fire time.
    talk_done_at: Vec<Option<f32>>,
    /// The active tutorial `gate`, if any: the world is frozen and the
    /// scenario clock stops until [`ScenarioState::gate_notify`] sees the
    /// gated input succeed.
    gate: Option<GateState>,
    /// Per step: the time its (last) `gate` released (`None` = not yet or no
    /// gate). `step_done` / `timer { after }` on a gated step wait for this.
    gate_done_at: Vec<Option<f32>>,
    /// A `checkpoint` action ran since the last
    /// [`ScenarioState::take_checkpoint_request`].
    checkpoint_requested: bool,
    /// Whether the player may fight (see [`Action::Combat`]). Default true.
    combat_enabled: bool,
}

/// Live state of a tutorial `gate`: what it waits for, which step owns it,
/// and the actions of that step still to run once it releases.
#[derive(Debug, Clone, Copy)]
struct GateState {
    def: GateDef,
    step_idx: usize,
    /// The owning step's actions AFTER the gate (run on release; may install
    /// the step's next gate, chaining within one step).
    rest: &'static [Action],
    /// Where the gated action happens (the step's spawned target / the
    /// pickup / the downed victim): the player is tethered near it by
    /// invisible walls ([`GATE_TETHER_RADIUS`]) while the gate holds.
    anchor: Option<Vec2>,
    /// Set by the first `tick` after installation, i.e. once a frame's
    /// systems have run UNDER this gate. `gate_notify` ignores an unarmed
    /// gate: the events of the frame that installed it were produced by
    /// unfrozen input and must not release it before its prompt was even
    /// drawn.
    armed: bool,
}

/// Live state of a `talk` conversation: the line on screen, the lines still
/// queued, and the panel's slide animation. Advanced by the player
/// ([`ScenarioState::dialogue_advance`]), not by the clock.
#[derive(Debug, Clone)]
struct DialogueState {
    current: TalkDef,
    queue: VecDeque<TalkDef>,
    /// Seconds the current line has been up (drives the typewriter).
    line_age: f32,
    /// Panel presence 0..1: rises to 1 while open, falls to 0 once `closing`.
    slide: f32,
    /// The last line was dismissed: the panel is sliding out.
    closing: bool,
    /// Indices of the steps whose `talk` lines joined this conversation.
    owners: Vec<usize>,
}

/// What the renderer needs to draw the dialogue panel this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DialogueView {
    pub who: &'static str,
    pub text: &'static str,
    /// Characters revealed by the typewriter so far.
    pub chars_shown: usize,
    pub fully_typed: bool,
    /// Panel presence 0..1, already smoothstepped (0 = off screen).
    pub slide: f32,
    /// More lines are queued after this one.
    pub more: bool,
}

/// Live state of a `hold` action.
#[derive(Debug, Clone, Copy, PartialEq)]
struct HoldState {
    def: HoldDef,
    /// Scenario time at which the hold ends (the cap when `until_comms_idle`).
    until: f32,
}

/// Live state of a `look_at` action.
#[derive(Debug, Clone, Copy, PartialEq)]
struct LookState {
    def: LookAtDef,
    start: f32,
}

/// Longest a `hold_until_comms_idle` may lock the player, seconds.
pub const HOLD_COMMS_IDLE_CAP: f32 = 20.0;
/// Ease-in / ease-out length of a `look_at` camera move, seconds.
pub const LOOK_AT_EASE_SECS: f32 = 0.6;
/// Slide-in / slide-out length of the dialogue panel, seconds.
pub const DIALOGUE_SLIDE_SECS: f32 = 0.25;
/// Typewriter speed of a dialogue line, characters per second (snappier than
/// the ambient comms feed; a press mid-line reveals the whole line).
pub const DIALOGUE_CHARS_PER_SEC: f32 = 55.0;

impl ScenarioState {
    pub fn new(floor: &'static FloorDef) -> Self {
        ScenarioState {
            floor,
            time: 0.0,
            fired_at: vec![None; floor.scenario.len()],
            auto_open_on_all_dead: !floor.has_exit_opener(),
            auto_opened: false,
            opened_exits: Vec::new(),
            objective: floor.objective.to_string(),
            comms: CommsFeed::default(),
            sfx: Vec::new(),
            hold: None,
            look: None,
            dialogue: None,
            talk_done_at: vec![None; floor.scenario.len()],
            gate: None,
            gate_done_at: vec![None; floor.scenario.len()],
            checkpoint_requested: false,
            combat_enabled: true,
        }
    }

    pub fn floor(&self) -> &'static FloorDef {
        self.floor
    }

    pub fn time(&self) -> f32 {
        self.time
    }

    /// Whether the step with this id has fired.
    pub fn step_fired(&self, id: &str) -> bool {
        self.floor
            .scenario
            .iter()
            .zip(&self.fired_at)
            .any(|(s, f)| s.id == id && f.is_some())
    }

    /// Ids of exits opened by the scenario so far.
    pub fn opened_exits(&self) -> &[&'static str] {
        &self.opened_exits
    }

    /// Take the pending one-shot sound effects.
    pub fn drain_sfx(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.sfx)
    }

    /// Whether a `hold` is locking the player's input right now.
    pub fn hold_active(&self) -> bool {
        self.hold.is_some()
    }

    /// The caption of the running `hold`, if it has one.
    pub fn hold_caption(&self) -> Option<&'static str> {
        self.hold.and_then(|h| h.def.text)
    }

    /// The running `look_at`: the world point and its weight 0..1 (eased in
    /// over [`LOOK_AT_EASE_SECS`], held, eased out over the last
    /// [`LOOK_AT_EASE_SECS`]). `None` when no look is running.
    pub fn look_at(&self) -> Option<(Vec2, f32)> {
        let l = self.look?;
        let w = look_at_weight(self.time - l.start, l.def.seconds);
        Some((Vec2::new(l.def.x, l.def.y), w))
    }

    /// Whether a `talk` conversation is up (the player is locked and paces
    /// it with click / Space / Enter). True through the slide-out too, so a
    /// dismissing click can never fire the weapon.
    pub fn dialogue_active(&self) -> bool {
        self.dialogue.is_some()
    }

    /// The dialogue panel to draw this frame, if a conversation is up.
    pub fn dialogue_view(&self) -> Option<DialogueView> {
        let d = self.dialogue.as_ref()?;
        let total = d.current.text.chars().count();
        let shown = ((d.line_age * DIALOGUE_CHARS_PER_SEC) as usize).min(total);
        let s = d.slide.clamp(0.0, 1.0);
        Some(DialogueView {
            who: d.current.who,
            text: d.current.text,
            chars_shown: shown,
            fully_typed: shown >= total,
            slide: s * s * (3.0 - 2.0 * s),
            more: !d.queue.is_empty(),
        })
    }

    /// Player input on an active conversation: reveal the current line if it
    /// is still typing, otherwise move to the next line, otherwise dismiss
    /// the panel (it slides out and control returns).
    pub fn dialogue_advance(&mut self) {
        let Some(d) = self.dialogue.as_mut() else {
            return;
        };
        if d.closing {
            return;
        }
        let typing_time = d.current.text.chars().count() as f32 / DIALOGUE_CHARS_PER_SEC;
        if d.line_age < typing_time {
            d.line_age = typing_time;
        } else if let Some(next) = d.queue.pop_front() {
            d.current = next;
            d.line_age = 0.0;
        } else {
            d.closing = true;
        }
    }

    /// The active tutorial gate, if any (the world must be frozen: the game
    /// loop runs only the player-driven systems and masks every input but
    /// the gated one — see `sim::gate_frozen_step`).
    pub fn gate_view(&self) -> Option<GateDef> {
        self.gate.map(|g| g.def)
    }

    /// Where the active gate's action happens, if the gate could tell: the
    /// tether centre for [`tether_player`].
    pub fn gate_anchor(&self) -> Option<Vec2> {
        self.gate.and_then(|g| g.anchor)
    }

    /// Whether the player may fight (fire / throw / punch / finisher). Gates
    /// bypass this: their one action stays allowed.
    pub fn combat_enabled(&self) -> bool {
        self.combat_enabled
    }

    /// Feed the frame's drained [`GameEvent`]s to the active gate: when one
    /// satisfies it, the gate releases — the owning step's remaining actions
    /// run (possibly installing the step's next gate) and the step counts as
    /// done for `step_done` / `timer { after }` chains.
    ///
    /// [`GameEvent`]: crate::components::GameEvent
    pub fn gate_notify(&mut self, world: &mut World, events: &[crate::components::GameEvent]) {
        let Some(g) = self.gate else { return };
        if !g.armed {
            return; // installed this frame: these events predate it
        }
        if events.iter().any(|e| g.def.input.satisfied_by(e)) {
            self.gate = None;
            self.gate_done_at[g.step_idx] = Some(self.time);
            self.run_actions(world, g.rest, g.step_idx);
        }
    }

    /// Debug escape hatch (the `?debug` **G** key): release the active gate
    /// as if its input had succeeded, so a mis-designed gate with no possible
    /// target can never softlock a session.
    pub fn gate_skip(&mut self, world: &mut World) {
        if let Some(g) = self.gate.take() {
            self.gate_done_at[g.step_idx] = Some(self.time);
            self.run_actions(world, g.rest, g.step_idx);
        }
    }

    /// Whether a `checkpoint` action ran since the last call: the game loop
    /// snapshots the world + this scenario when it returns true.
    pub fn take_checkpoint_request(&mut self) -> bool {
        std::mem::take(&mut self.checkpoint_requested)
    }

    /// Queue one `talk` line (from step `step_idx`). Starts a conversation if
    /// none is up; otherwise appends to the running one — so consecutive
    /// `talk` actions (and same-tick steps) form a single conversation.
    fn enqueue_talk(&mut self, line: TalkDef, step_idx: usize) {
        match self.dialogue.as_mut() {
            Some(d) => {
                d.queue.push_back(line);
                if d.closing {
                    // The panel was on its way out: reopen on the new line.
                    d.closing = false;
                    d.current = d.queue.pop_front().unwrap();
                    d.line_age = 0.0;
                }
                if !d.owners.contains(&step_idx) {
                    d.owners.push(step_idx);
                }
            }
            None => {
                self.dialogue = Some(DialogueState {
                    current: line,
                    queue: VecDeque::new(),
                    line_age: 0.0,
                    slide: 0.0,
                    closing: false,
                    owners: vec![step_idx],
                });
            }
        }
    }

    /// Advance the `hold` / `look_at` / dialogue clocks (called from `tick`).
    fn tick_beats(&mut self, dt: f32) {
        if let Some(h) = self.hold {
            let comms_idle = !self.comms.is_active(self.time);
            let done = self.time >= h.until || (h.def.until_comms_idle && comms_idle);
            if done {
                self.hold = None;
            }
        }
        if let Some(l) = self.look {
            if self.time - l.start >= l.def.seconds {
                self.look = None;
            }
        }
        let mut finished = false;
        if let Some(d) = self.dialogue.as_mut() {
            d.line_age += dt;
            let step = dt / DIALOGUE_SLIDE_SECS;
            if d.closing {
                d.slide -= step;
                finished = d.slide <= 0.0;
            } else {
                d.slide = (d.slide + step).min(1.0);
            }
        }
        if finished {
            if let Some(d) = self.dialogue.take() {
                for i in d.owners {
                    self.talk_done_at[i] = Some(self.time);
                }
            }
        }
    }

    /// Advance the scenario by `dt`: fire due steps (each once), run their
    /// actions on the world, and advance the comms feed.
    ///
    /// Within one tick, steps whose triggers depend on the rogue counts
    /// (`kills`, `all_dead`) are evaluated *after* the other steps of the
    /// same pass, and the counts are recomputed after every fired step, so a
    /// `spawn` in the same tick can never let `all_dead` slip through.
    pub fn tick(&mut self, world: &mut World, dt: f32) {
        // A tutorial gate freezes the whole scenario: the clock and every
        // trigger hold their breath (timers must not advance) until the
        // gated input succeeds ([`ScenarioState::gate_notify`]). Only the
        // comms typewriter keeps playing — the lines a gated step queued
        // BEFORE its gate (the beat's flavour) still type out under the
        // prompt; the frozen clock keeps delayed lines waiting.
        if let Some(g) = self.gate.as_mut() {
            // This frame's systems ran frozen under the gate: from now on
            // their events may release it.
            g.armed = true;
            self.comms.update(self.time, dt);
            return;
        }
        self.time += dt;

        let player_pos = world
            .first::<Player>()
            .and_then(|p| world.get_component::<Position>(p))
            .map(|p| p.to_vec2());
        let mut counts = count_rogues(world);
        let boss_dead = any_boss_dead(world);
        let extracted = ElevatorSystem::extraction(world).is_some();

        // Fire steps until nothing new fires this tick (chained `step_done`
        // triggers resolve within the same frame).
        loop {
            let mut fired_any = false;
            // Pass 0: everything but the count-based triggers (may spawn);
            // pass 1: `kills` / `all_dead` against the fresh counts.
            for count_pass in [false, true] {
                for i in 0..self.floor.scenario.len() {
                    if self.fired_at[i].is_some() {
                        continue;
                    }
                    let step = &self.floor.scenario[i];
                    if step.trigger.is_count_based() != count_pass {
                        continue;
                    }
                    let (kills, alive) = counts;
                    let ctx = TriggerCtx {
                        player_pos,
                        kills,
                        alive,
                        boss_dead,
                        extracted,
                    };
                    if self.trigger_holds(step.trigger, &ctx) {
                        self.fired_at[i] = Some(self.time);
                        self.run_actions(world, step.actions, i);
                        counts = count_rogues(world);
                        fired_any = true;
                        if self.gate.is_some() {
                            // A gate just installed: the world (and this
                            // scenario) freeze mid-tick. Nothing else fires
                            // until the gate releases.
                            return;
                        }
                    }
                }
            }
            if !fired_any {
                break;
            }
        }

        // Legacy floors: all rogues dead opens every exit (checked against the
        // counts *after* this tick's spawns).
        if self.auto_open_on_all_dead && !self.auto_opened && counts.1 == 0 {
            self.auto_opened = true;
            for exit in self.floor.exits {
                self.set_exit_open(world, exit.id, true);
            }
        }

        self.comms.update(self.time, dt);
        self.tick_beats(dt);
    }

    fn trigger_holds(&self, trigger: Trigger, ctx: &TriggerCtx) -> bool {
        match trigger {
            Trigger::Start => true,
            Trigger::EnterZone { zone, before } => {
                // `before`: disarmed forever once that other step fires.
                if before.is_some_and(|id| self.fired_time(id).is_some()) {
                    return false;
                }
                match (ctx.player_pos, self.floor.zone(zone)) {
                    (Some(p), Some(z)) => z.rect.contains(p),
                    _ => false,
                }
            }
            Trigger::Kills(n) => ctx.kills >= n,
            // "Every rogue is dead" needs a rogue to have died: a floor whose
            // hostiles have not shown up yet (an un-alerted crowd, a wave
            // still to spawn) is not "cleared".
            Trigger::AllDead => ctx.alive == 0 && ctx.kills > 0,
            Trigger::Timer { seconds, after } => {
                let base = match after {
                    None => Some(0.0),
                    Some(id) => self.after_time(id),
                };
                match base {
                    Some(t0) => self.time - t0 >= seconds,
                    None => false,
                }
            }
            Trigger::ExitOpen(None) => !self.opened_exits.is_empty(),
            Trigger::ExitOpen(Some(id)) => self.opened_exits.contains(&id),
            Trigger::StepDone(id) => self.step_done_time(id).is_some(),
            Trigger::BossDead => ctx.boss_dead,
            Trigger::Extracted => ctx.extracted,
        }
    }

    fn fired_time(&self, id: &str) -> Option<f32> {
        self.floor
            .scenario
            .iter()
            .zip(&self.fired_at)
            .find(|(s, _)| s.id == id)
            .and_then(|(_, f)| *f)
    }

    /// When step `id` counts as DONE for a `step_done` trigger: for a step
    /// with `gate` actions, the moment its (last) gate released (`None`
    /// while it is pending — triggers are never evaluated while a gate is
    /// active, so a released gate here is the step's final one); for any
    /// other step, the moment it fired.
    fn step_done_time(&self, id: &str) -> Option<f32> {
        let (i, step) = self
            .floor
            .scenario
            .iter()
            .enumerate()
            .find(|(_, s)| s.id == id)?;
        let fired = self.fired_at[i]?;
        if step.actions.iter().any(|a| matches!(a, Action::Gate(_))) {
            self.gate_done_at[i]
        } else {
            Some(fired)
        }
    }

    /// The base time a `timer { after: id }` counts from: for a step with
    /// `gate` actions, the moment its gate released; for a step with `talk`
    /// actions, the moment its conversation ENDED (panel dismissed; `None`
    /// while it is still up — the player paces it); for any other step, the
    /// moment it fired.
    fn after_time(&self, id: &str) -> Option<f32> {
        let (i, step) = self
            .floor
            .scenario
            .iter()
            .enumerate()
            .find(|(_, s)| s.id == id)?;
        let fired = self.fired_at[i]?;
        if step.actions.iter().any(|a| matches!(a, Action::Gate(_))) {
            self.gate_done_at[i]
        } else if step.actions.iter().any(|a| matches!(a, Action::Talk(_))) {
            self.talk_done_at[i]
        } else {
            Some(fired)
        }
    }

    fn run_actions(&mut self, world: &mut World, actions: &'static [Action], step_idx: usize) {
        // The last position this step spawned at: the default gate anchor
        // (tutorial steps spawn their victim just before their gate).
        let mut last_spawn: Option<Vec2> = None;
        for (i, action) in actions.iter().enumerate() {
            match *action {
                Action::Gate(def) => {
                    // Install the gate and STOP: the remaining actions run
                    // when it releases (`gate_notify`), so gates chain
                    // within one step. One gate at a time by construction —
                    // the scenario clock halts while one is active, so no
                    // other step can fire under it.
                    // No anchor for `pickup` gates: fetching IS a walk (and
                    // corpses drop weapons, so "nearest pickup" often points
                    // at the wrong thing).
                    let anchor = last_spawn.or_else(|| match def.input {
                        GateInput::Finish => nearest_stunned_to_player(world),
                        _ => None,
                    });
                    self.gate = Some(GateState {
                        def,
                        step_idx,
                        rest: &actions[i + 1..],
                        anchor,
                        armed: false,
                    });
                    return;
                }
                Action::Checkpoint => self.checkpoint_requested = true,
                Action::Combat(on) => self.combat_enabled = on,
                Action::Disarm => {
                    if let Some(p) = world.first::<Player>() {
                        world.remove_component::<crate::components::Weapon>(p);
                    }
                }
                Action::Say(say) => {
                    self.comms.enqueue(say.who, say.text, self.time + say.delay);
                }
                Action::Talk(line) => self.enqueue_talk(line, step_idx),
                Action::Spawn(spawns) => {
                    for s in spawns {
                        spawn_from_def(world, s);
                    }
                    last_spawn = spawns.last().map(|s| Vec2::new(s.x, s.y));
                }
                Action::OpenExit(id) => self.set_exit_open(world, id, true),
                Action::CloseExit(id) => self.set_exit_open(world, id, false),
                Action::Objective(text) => self.objective = text.to_string(),
                Action::Sfx(name) => self.sfx.push(name),
                Action::Alert(target) => {
                    crate::systems::passive::alert_passives(world, target);
                }
                Action::Hold(def) => {
                    let secs = if def.until_comms_idle {
                        def.seconds.min(HOLD_COMMS_IDLE_CAP)
                    } else {
                        def.seconds
                    };
                    self.hold = Some(HoldState {
                        def,
                        until: self.time + secs,
                    });
                }
                Action::LookAt(def) => {
                    self.look = Some(LookState {
                        def,
                        start: self.time,
                    });
                }
            }
        }
    }

    fn set_exit_open(&mut self, world: &mut World, id: &'static str, open: bool) {
        for entity in world.query::<Elevator>() {
            if let Some(elev) = world.get_component_mut::<Elevator>(entity) {
                if elev.is_exit && elev.id == id {
                    let changed = elev.open != open;
                    elev.open = open;
                    if changed && open {
                        self.sfx.push("elevator");
                    }
                }
            }
        }
        if open && !self.opened_exits.contains(&id) {
            self.opened_exits.push(id);
        }
    }
}

/// Weight 0..1 of a `look_at` that started `elapsed` seconds ago and lasts
/// `total`: eases in over [`LOOK_AT_EASE_SECS`], holds, eases out over the
/// last [`LOOK_AT_EASE_SECS`] (smoothstep both ways). Short looks scale the
/// ramps down so they still peak.
pub fn look_at_weight(elapsed: f32, total: f32) -> f32 {
    if total <= 0.0 || elapsed < 0.0 || elapsed >= total {
        return 0.0;
    }
    let ramp = LOOK_AT_EASE_SECS.min(total / 2.0);
    let t = if elapsed < ramp {
        elapsed / ramp
    } else if elapsed > total - ramp {
        (total - elapsed) / ramp
    } else {
        1.0
    };
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
