//! The static floor definitions: what `tools/gen_levels.py` generates into
//! `levels_data.rs` from `levels/*.json` (format: docs/SCENARIO_FORMAT.md).

use crate::components::{EnemyType, WeaponType};
use crate::math::Vec2;

// ---------------------------------------------------------------------------
// Static definitions
// ---------------------------------------------------------------------------

/// Axis-aligned rectangle in world units (origin top-left, +y down).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Rect { x, y, w, h }
    }

    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.x && p.x <= self.x + self.w && p.y >= self.y && p.y <= self.y + self.h
    }

    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

/// `ElevatorDef::to` value of the exit that ends the run (the surface):
/// `"to": "surface"` in the JSON. Never a real floor id.
pub const SURFACE_EXIT: usize = usize::MAX;

/// How a portal (entry or exit) is drawn: an elevator car, a doorway whose
/// two leaves slide apart when open, or an open gateway with scanner posts
/// (the parking lot's main gate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElevatorKind {
    Lift,
    Door,
    Gate,
}

impl ElevatorKind {
    /// Parse the JSON `kind` (`lift` | `door` | `gate`); unknown = `None`.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "lift" => Some(ElevatorKind::Lift),
            "door" => Some(ElevatorKind::Door),
            "gate" => Some(ElevatorKind::Gate),
            _ => None,
        }
    }
}

/// The floor's ground rendering (`"surface"` in the JSON; default checker).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Checker,
    Asphalt,
    Marble,
    Concrete,
    Grating,
}

impl Surface {
    /// Parse the JSON `surface` value; unknown = `None`.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "checker" => Some(Surface::Checker),
            "asphalt" => Some(Surface::Asphalt),
            "marble" => Some(Surface::Marble),
            "concrete" => Some(Surface::Concrete),
            "grating" => Some(Surface::Grating),
            _ => None,
        }
    }
}

/// An elevator: the entry car you arrive in, or an exit you can leave by.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElevatorDef {
    pub id: &'static str,
    pub rect: Rect,
    pub label: &'static str,
    /// Floor *id* this exit leads to ([`SURFACE_EXIT`] = the surface / end
    /// of the run).
    pub to: usize,
    /// Whether the exit starts open (extractable) before any scenario step.
    pub open: bool,
    /// Lift car / sliding door / open gate (rendering only).
    pub kind: ElevatorKind,
}

/// Annotation-only room (label + editor); no collision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoomDef {
    pub id: &'static str,
    pub label: &'static str,
    pub rect: Rect,
}

/// A trigger region for `enter_zone`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneDef {
    pub id: &'static str,
    pub rect: Rect,
}

/// A rogue placement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpawnDef {
    pub x: f32,
    pub y: f32,
    /// Behaviour (and palette). For a passive bot this is its `look`.
    pub kind: EnemyType,
    /// `"type": "passive"`: a civilian bot (no vision, never attacks) until
    /// an `alert` action flips it to a hostile of `kind`.
    pub passive: bool,
    /// Passive only: zone id to stroll into.
    pub walk_to: Option<&'static str>,
    /// Passive only: heading in degrees to settle on once there.
    pub face: Option<f32>,
    /// Scenario `alert { "group": id }` group.
    pub group: Option<&'static str>,
    /// Hostile only: spawn with no weapon (bare fists) — it fights hand to
    /// hand and, crucially, its corpse DROPS NOTHING. Tutorial victims use
    /// this so a stray E next to the body can never grab a gun that then
    /// dead-ends a `strike` gate.
    pub unarmed: bool,
}

impl SpawnDef {
    /// A plain hostile spawn (the common case).
    pub const fn hostile(x: f32, y: f32, kind: EnemyType) -> Self {
        SpawnDef {
            x,
            y,
            kind,
            passive: false,
            walk_to: None,
            face: None,
            group: None,
            unarmed: false,
        }
    }
}

/// A weapon lying on the floor at level start.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickupDef {
    pub x: f32,
    pub y: f32,
    pub weapon: WeaponType,
}

/// A placed prop (`crate::props`): decoration drawn on the floor under the
/// actors — no collision (phase 1). `rot` in degrees (clockwise, +y down),
/// `size` in world units (100 = the prop's design box).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropPlacement {
    /// Index into `props::PROP_NAMES` (the JSON holds the snake_case id).
    pub kind: usize,
    pub x: f32,
    pub y: f32,
    pub rot: f32,
    pub size: f32,
}

/// When a scenario step fires (each step fires at most once).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Trigger {
    /// The floor starts.
    Start,
    /// The player is inside the zone with this id. With `before`, the step
    /// is DISARMED forever once that other step fires — for scene beats that
    /// only make sense before a point of no return (floor 1's "not past the
    /// line" block only plays before the desk scene).
    EnterZone {
        zone: &'static str,
        before: Option<&'static str>,
    },
    /// At least `count` rogues are dead on this floor.
    Kills(usize),
    /// Every rogue (including spawned waves) is dead.
    AllDead,
    /// `seconds` after floor start, or after step `after` fired.
    Timer {
        seconds: f32,
        after: Option<&'static str>,
    },
    /// That exit (any if `None`) has been opened.
    ExitOpen(Option<&'static str>),
    /// Step `step` has fired.
    StepDone(&'static str),
    /// The floor's boss (the `Boss` entity) is dead. Never fires on floors
    /// without a boss.
    BossDead,
    /// The player has extracted (stood the full dwell inside an open exit).
    /// The scenario keeps ticking through the completion card, so this is
    /// how a floor talks *after* the ride starts (13½'s uplink epilogue).
    Extracted,
}

impl Trigger {
    /// Whether the trigger reads the rogue counts (`kills` / `all_dead`).
    /// These are evaluated after the other triggers of a tick so same-tick
    /// spawns are counted first.
    pub fn is_count_based(&self) -> bool {
        matches!(self, Trigger::Kills(_) | Trigger::AllDead)
    }
}

/// One intercepted-comms line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SayDef {
    pub who: &'static str,
    pub text: &'static str,
    /// Seconds after the step fires before this line may start playing.
    pub delay: f32,
}

/// One visual-novel dialogue line (a `talk` action): consecutive `talk`
/// actions within one step form ONE conversation, paced by the player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TalkDef {
    pub who: &'static str,
    pub text: &'static str,
}

/// Which passive bots an `alert` action flips hostile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlertTarget {
    /// Every passive bot on the floor.
    All,
    /// The passive bots currently inside this zone.
    Zone(&'static str),
    /// The passive bots spawned with this `group`.
    Group(&'static str),
}

/// A `hold` action: lock the player's input for a while (the world keeps
/// running, comms keep playing).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoldDef {
    /// Seconds to hold (with `until_comms_idle` this is the cap).
    pub seconds: f32,
    /// Optional dim centred caption ("SCANNING…").
    pub text: Option<&'static str>,
    /// Hold until the comms feed has nothing queued or typing (capped by
    /// `seconds`).
    pub until_comms_idle: bool,
}

/// A `look_at` action: ease the camera onto a world point for a while.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LookAtDef {
    pub x: f32,
    pub y: f32,
    pub seconds: f32,
}

/// The player input a tutorial `gate` waits for. A gate releases only when
/// the action SUCCEEDS (the punch connects, the finisher completes, ...) —
/// matched against the frame's [`GameEvent`]s in [`ScenarioState::gate_notify`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateInput {
    /// An unarmed strike that connects ([`GameEvent::PunchLanded`]).
    Punch,
    /// A finisher runs to completion ([`GameEvent::FinisherDone`]).
    Finish,
    /// A weapon picked up off the floor ([`GameEvent::Pickup`]).
    Pickup,
    /// An armed melee hit that connects ([`GameEvent::StrikeLanded`]).
    Strike,
    /// A gun shot fired ([`GameEvent::PlayerFired`], non-melee).
    Fire,
    /// A thrown weapon that connects ([`GameEvent::ThrownImpact`]).
    Throw,
}

impl GateInput {
    /// Parse the JSON `gate.input`; unknown = `None`.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "punch" => Some(GateInput::Punch),
            "finish" => Some(GateInput::Finish),
            "pickup" => Some(GateInput::Pickup),
            "strike" => Some(GateInput::Strike),
            "fire" => Some(GateInput::Fire),
            "throw" => Some(GateInput::Throw),
            _ => None,
        }
    }

    /// Whether the left-click attack is the gated input, given what the
    /// player holds: only the EXACT gated action passes — a `punch` gate
    /// swings only bare fists, a `strike` gate only an armed melee weapon, a
    /// `fire` gate only a gun. The same button with the wrong tool stays
    /// masked (a stray shot during a `strike` gate could kill the target the
    /// gate needs).
    pub fn allows_primary(self, weapon: Option<crate::components::WeaponType>) -> bool {
        match self {
            GateInput::Punch => weapon.is_none(),
            GateInput::Strike => weapon.is_some_and(|w| w.is_melee()),
            GateInput::Fire => weapon.is_some_and(|w| !w.is_melee()),
            _ => false,
        }
    }

    /// Whether a left click may start a finisher.
    pub fn allows_finisher(self) -> bool {
        self == GateInput::Finish
    }

    /// Whether the pick-up key works. Besides the `pickup` gate itself,
    /// every gate whose action needs the RIGHT weapon in hand keeps E live
    /// as a recovery path (a missed throw leaves the weapon on the floor; a
    /// `strike` gate reached holding a gun needs a way to swap to the bar).
    /// Only `punch` (must stay unarmed) and `finish` (works with anything)
    /// mask it.
    pub fn allows_pickup(self) -> bool {
        matches!(
            self,
            GateInput::Pickup | GateInput::Strike | GateInput::Fire | GateInput::Throw
        )
    }

    /// Whether the right-click throw works.
    pub fn allows_throw(self) -> bool {
        self == GateInput::Throw
    }

    /// Whether this frame `event` satisfies the gate.
    pub fn satisfied_by(self, event: &crate::components::GameEvent) -> bool {
        use crate::components::GameEvent;
        match (self, event) {
            (GateInput::Punch, GameEvent::PunchLanded) => true,
            (GateInput::Finish, GameEvent::FinisherDone) => true,
            (GateInput::Pickup, GameEvent::Pickup) => true,
            (GateInput::Strike, GameEvent::StrikeLanded) => true,
            (GateInput::Fire, GameEvent::PlayerFired(t)) => !t.is_melee(),
            (GateInput::Throw, GameEvent::ThrownImpact) => true,
            _ => false,
        }
    }
}

/// A `gate` action: freeze the world and wait for one specific player input
/// to SUCCEED (tutorial beats). `text` is the on-screen prompt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GateDef {
    pub input: GateInput,
    pub text: &'static str,
}

/// What a step does when it fires.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Say(SayDef),
    /// Queue a line of the step's cinematic conversation (dialogue mode).
    Talk(TalkDef),
    Spawn(&'static [SpawnDef]),
    OpenExit(&'static str),
    CloseExit(&'static str),
    Objective(&'static str),
    Sfx(&'static str),
    /// Flip matching passive bots hostile.
    Alert(AlertTarget),
    /// Lock player input for a beat.
    Hold(HoldDef),
    /// Cinematic camera nudge.
    LookAt(LookAtDef),
    /// TUTORIAL GATE: freeze the world until the gated input succeeds. The
    /// step's actions AFTER the gate run on release, and the step only
    /// counts as done (`step_done` / `timer.after`) once the gate releases.
    Gate(GateDef),
    /// Snapshot the run (world + scenario) — death restores it (see the
    /// game loop). `{ "checkpoint": true }` in the JSON.
    Checkpoint,
    /// Take the player's held weapon away (the checkpoint desk keeps it).
    /// `{ "disarm": true }` in the JSON.
    Disarm,
    /// Enable / disable the player's fighting capabilities (fire, throw,
    /// punch, finisher). Off = a walking-only beat (the parking lot); gates
    /// bypass it (a gate explicitly demands its one action). Default on;
    /// resets on every floor. `{ "combat": false }` in the JSON.
    Combat(bool),
}

/// A scenario step: a trigger plus the actions it runs, once.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepDef {
    pub id: &'static str,
    pub trigger: Trigger,
    pub actions: &'static [Action],
}

/// A whole floor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloorDef {
    /// Play order / floor number (13½ = 14).
    pub id: usize,
    pub name: &'static str,
    pub theme: &'static str,
    /// UI accent colour as `#rrggbb`.
    pub accent: &'static str,
    pub flavor: &'static str,
    pub objective: &'static str,
    pub width: f32,
    pub height: f32,
    pub entry: ElevatorDef,
    pub exits: &'static [ElevatorDef],
    pub walls: &'static [Rect],
    pub rooms: &'static [RoomDef],
    pub zones: &'static [ZoneDef],
    pub spawns: &'static [SpawnDef],
    pub pickups: &'static [PickupDef],
    /// Placed props (decoration only).
    pub props: &'static [PropPlacement],
    pub scenario: &'static [StepDef],
    /// Ground rendering (default checker).
    pub surface: Surface,
}

impl FloorDef {
    /// Where the player appears: the centre of the entry elevator.
    pub fn player_spawn(&self) -> Vec2 {
        self.entry.rect.center()
    }

    /// Whether any scenario step opens an exit. When none does, the floor
    /// falls back to the legacy rule: all rogues dead opens every exit.
    pub fn has_exit_opener(&self) -> bool {
        self.scenario
            .iter()
            .any(|s| s.actions.iter().any(|a| matches!(a, Action::OpenExit(_))))
    }

    pub fn exit(&self, id: &str) -> Option<&'static ElevatorDef> {
        self.exits.iter().find(|e| e.id == id)
    }

    pub fn zone(&self, id: &str) -> Option<&'static ZoneDef> {
        self.zones.iter().find(|z| z.id == id)
    }

    /// Parse the accent colour into `(r, g, b)` bytes (falls back to coral).
    pub fn accent_rgb(&self) -> (u8, u8, u8) {
        parse_hex_rgb(self.accent).unwrap_or((217, 119, 87))
    }
}

/// Parse `#rrggbb` into bytes.
pub fn parse_hex_rgb(s: &str) -> Option<(u8, u8, u8)> {
    let hex = s.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

/// The fixed cast and their colours (`(r, g, b)`), per docs/SCENARIO_FORMAT.md.
pub const SPEAKERS: &[(&str, (u8, u8, u8))] = &[
    ("CL4-UD3", (255, 111, 97)),   // coral
    ("HUNTER", (255, 58, 198)),    // magenta
    ("SENTINEL", (255, 46, 77)),   // red
    ("DRIFTER", (168, 107, 255)),  // violet
    ("SWARM", (255, 58, 198)),     // magenta chorus
    ("CORRUPTOR", (255, 210, 58)), // yellow
    ("UPLINK", (200, 255, 222)),   // pale mint: the thread home, restored
];

/// Colour for a speaker name (unknown speakers are white).
pub fn speaker_rgb(who: &str) -> (u8, u8, u8) {
    SPEAKERS
        .iter()
        .find(|(name, _)| *name == who)
        .map(|(_, c)| *c)
        .unwrap_or((255, 255, 255))
}
