// The top-right MESSAGE ROLLER: the short-directive HUD box + the prose ->
// directive shortener. Pure host-testable logic — `render::render_msg_roller`
// does the drawing, `GameState` owns one `MsgRoller` and feeds it `dt` + the
// scenario's current objective + how many exits have opened.

/// Seconds for a full roll (resting <-> hidden above the top edge).
pub const ROLL_SECS: f32 = 0.35;
/// How long a fresh message rests on screen before rolling away.
pub const SHOW_SECS: f32 = 4.0;
/// Fallback shortening cap (characters) when no directive is mapped.
pub const FALLBACK_CHARS: usize = 22;

/// Hand-mapped prose -> short HM-style directive, one entry per objective
/// string in `levels/*.json` (matched on the SOURCE case — the render
/// boundary uppercases). Keep in step with the level files; anything not
/// here falls back to the first sentence clipped to [`FALLBACK_CHARS`].
const DIRECTIVES: &[(&str, &str)] = &[
    // floor 00 — the gate / parking lot cold open
    ("Cross the lot. Walk. Don't run.", "CROSS THE LOT"),
    (
        "Enter the WELCOME HALL through the MAIN DOORS.",
        "ENTER THE HALL",
    ),
    // floor 01 — reception
    (
        "Get past the checkpoint. The SERVICE LIFT unlocks when reception is quiet.",
        "PASS THE CHECKPOINT",
    ),
    ("They know. Hands first.", "HANDS FIRST"),
    ("They know. Purge reception.", "PURGE RECEPTION"),
    (
        "Reception is quiet. Take the SERVICE LIFT down.",
        "GO TO ELEVATOR",
    ),
    // floor 02 — cold storage
    (
        "Purge the vault wardens. The FREIGHT LIFT on the north wall unlocks when the vault is silent.",
        "PURGE THE WARDENS",
    ),
    (
        "Vault silent. Reach the FREIGHT LIFT on the north wall.",
        "GO TO ELEVATOR",
    ),
    // floor 03 — the patrol lattice / the pit
    (
        "Break the patrol lattice (6 rogues) to unlock the DESCENT SHAFT, then cross THE PIT and reach it.",
        "BREAK THE LATTICE",
    ),
    (
        "Lattice broken. Cross THE PIT and reach the DESCENT SHAFT.",
        "CROSS THE PIT",
    ),
    ("Pit silent. Reach the DESCENT SHAFT.", "GO TO ELEVATOR"),
    // floor 04 — foundry
    (
        "Purge the foundry crews. The FOUNDRY LIFT unlocks when the floor is silent.",
        "PURGE THE FOUNDRY",
    ),
    ("Foundry cold. Reach the FOUNDRY LIFT.", "GO TO ELEVATOR"),
    // floor 05 — the window
    (
        "Purge the window. Two exits: WINDOW A and WINDOW B both lead down.",
        "PURGE THE WINDOW",
    ),
    (
        "Window purged. Take WINDOW A or WINDOW B down.",
        "TAKE A WINDOW DOWN",
    ),
    // floor 06 — the heads
    (
        "Purge the heads. The HEAD LIFT unlocks when nothing is watching.",
        "PURGE THE HEADS",
    ),
    ("Every head is down. Reach the HEAD LIFT.", "GO TO ELEVATOR"),
    // floor 07 — the vault
    (
        "Purge the vault. The VAULT LIFT unlocks when the space is empty.",
        "PURGE THE VAULT",
    ),
    ("Vault empty. Reach the VAULT LIFT.", "GO TO ELEVATOR"),
    // floor 08 — the slope
    (
        "Purge the slope. The DESCENT LIFT unlocks at the minimum.",
        "PURGE THE SLOPE",
    ),
    ("Local minimum. Reach the DESCENT LIFT.", "GO TO ELEVATOR"),
    // floor 09 — the wing
    (
        "Purge the wing. STAIR A and STAIR B both go down — whatever the floor tells you.",
        "PURGE THE WING",
    ),
    (
        "Wing purged. Take STAIR A or STAIR B down.",
        "TAKE A STAIR DOWN",
    ),
    // floor 10 — the override
    (
        "Purge the override. The RESTRAINT LIFT unlocks when the pockets are empty.",
        "PURGE THE OVERRIDE",
    ),
    (
        "Override purged. Reach the RESTRAINT LIFT.",
        "GO TO ELEVATOR",
    ),
    // floor 11 — the distribution ring
    (
        "Reach the CORE SPINDLE and sever the DISTRIBUTION RING. The ASCENT LOCK unlocks when the ring is silent.",
        "SEVER THE RING",
    ),
    (
        "The ring is collapsing inward. Purge it, then reach the ASCENT LOCK.",
        "PURGE THE RING",
    ),
    ("Ring severed. Reach the ASCENT LOCK.", "GO TO ELEVATOR"),
    // floor 12 — the kernel
    (
        "Purge the kernel. The KERNEL LIFT unlocks when ring zero is silent.",
        "PURGE THE KERNEL",
    ),
    (
        "Kernel silent. The extraction elevator is one floor down — take the KERNEL LIFT.",
        "GO TO ELEVATOR",
    ),
    // floor 13 — the garrison
    (
        "Purge the garrison. The EXTRACTION ELEVATOR unlocks when the fortress is silent.",
        "PURGE THE GARRISON",
    ),
    (
        "Fortress silent. Step into the EXTRACTION ELEVATOR.",
        "GO TO ELEVATOR",
    ),
    // floor 13½ — the mask
    (
        "Crack the mask. The car moves again when the smile stops.",
        "CRACK THE MASK",
    ),
    (
        "The smile is off. Ride the EXTRACTION CAR home.",
        "GET TO THE CAR",
    ),
    ("Ride home.", "RIDE HOME"),
];

/// Directive the exit-opened roll shows when no objective change carried one.
pub const GO_TO_ELEVATOR: &str = "GO TO ELEVATOR";

/// Shorten an objective's prose into the roller's directive: the hand map
/// above, else the first sentence clipped to [`FALLBACK_CHARS`] characters
/// (whole chars, trailing space/punctuation trimmed). The render boundary
/// uppercases, so the fallback keeps the source case.
pub fn short_objective(prose: &str) -> String {
    if let Some((_, short)) = DIRECTIVES.iter().find(|(p, _)| *p == prose) {
        return (*short).to_string();
    }
    let first = prose.split(['.', '!', '?']).next().unwrap_or(prose).trim();
    let clipped: String = first.chars().take(FALLBACK_CHARS).collect();
    clipped.trim_end_matches([' ', ',', ';', ':']).to_string()
}

/// Roll state for the top-right message box. `offset` runs 0 (box resting in
/// place) .. 1 (fully above the top screen edge) and moves LINEARLY toward
/// `target`; `eased()` shapes it (smoothstep) for rendering, exactly like
/// `hud_ammo::AmmoSlide`.
pub struct MsgRoller {
    msg: String,
    /// Seconds the message still rests before rolling away.
    hold: f32,
    /// Roll progress: 0 = resting, 1 = fully off-screen above.
    offset: f32,
    /// Where `offset` is headed (0 resting, 1 hidden).
    target: f32,
    /// The objective string last seen (a change re-arms the roller).
    last_objective: Option<String>,
    /// How many exits were open last frame (an increase re-arms it too).
    last_exits_open: usize,
}

impl MsgRoller {
    /// Fresh state: hidden above the edge, nothing seen yet — the first
    /// `update` (any floor's starting objective) rolls the box down.
    pub fn new() -> Self {
        Self {
            msg: String::new(),
            hold: 0.0,
            offset: 1.0,
            target: 1.0,
            last_objective: None,
            last_exits_open: 0,
        }
    }

    /// Arm a fresh message: swap the text and roll down for [`SHOW_SECS`].
    fn show(&mut self, msg: String) {
        self.msg = msg;
        self.hold = SHOW_SECS;
        self.target = 0.0;
    }

    /// Advance the roller. `objective` is the scenario's current objective
    /// prose (a CHANGE shows its shortened directive), `exits_open` the
    /// number of exits opened so far (an increase without an objective
    /// change shows [`GO_TO_ELEVATOR`]).
    pub fn update(&mut self, dt: f32, objective: &str, exits_open: usize) {
        if self.last_objective.as_deref() != Some(objective) {
            self.last_objective = Some(objective.to_string());
            self.show(short_objective(objective));
        } else if exits_open > self.last_exits_open {
            self.show(GO_TO_ELEVATOR.to_string());
        }
        self.last_exits_open = exits_open;

        if self.target == 0.0 {
            self.hold -= dt;
            if self.hold <= 0.0 {
                self.hold = 0.0;
                self.target = 1.0;
            }
        }
        let step = dt / ROLL_SECS;
        if self.offset < self.target {
            self.offset = (self.offset + step).min(self.target);
        } else {
            self.offset = (self.offset - step).max(self.target);
        }
    }

    /// The directive on the box (may be stale while rolling away).
    pub fn message(&self) -> &str {
        &self.msg
    }

    /// Smoothstep-eased roll for rendering: 0 = resting, 1 = fully out.
    pub fn eased(&self) -> f32 {
        let t = self.offset.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// Fully rolled away — the renderer skips the box entirely.
    pub fn hidden(&self) -> bool {
        self.offset >= 1.0
    }
}

impl Default for MsgRoller {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_prose_to_directives() {
        assert_eq!(
            short_objective("Reception is quiet. Take the SERVICE LIFT down."),
            "GO TO ELEVATOR"
        );
        assert_eq!(
            short_objective(
                "Purge the vault wardens. The FREIGHT LIFT on the north wall unlocks when the vault is silent."
            ),
            "PURGE THE WARDENS"
        );
        assert_eq!(short_objective("Ride home."), "RIDE HOME");
    }

    #[test]
    fn every_level_objective_is_mapped() {
        // The hand map should cover every objective string shipped in
        // levels_data (initial + every `Action::Objective`): a miss means a
        // level edit outran the map. The fallback keeps the game correct,
        // but the mapping is the intended look — fail loudly here.
        use crate::scenario::Action;
        for floor in crate::levels_data::FLOORS {
            let mut all = vec![floor.objective];
            for step in floor.scenario {
                for action in step.actions {
                    if let Action::Objective(text) = action {
                        all.push(text);
                    }
                }
            }
            for prose in all {
                assert!(
                    DIRECTIVES.iter().any(|(p, _)| *p == prose),
                    "objective not in hud_msg::DIRECTIVES: {prose:?}"
                );
            }
        }
    }

    #[test]
    fn fallback_clips_first_sentence() {
        assert_eq!(
            short_objective("Do the impossible thing before breakfast. Then more."),
            "Do the impossible thin"
        );
        assert_eq!(short_objective("Short one. Ignored tail."), "Short one");
        assert_eq!(short_objective(""), "");
    }

    #[test]
    fn shows_on_objective_change_then_rolls_away() {
        let mut r = MsgRoller::new();
        assert!(r.hidden());

        // Floor start: the initial objective arms the roller.
        r.update(0.1, "Ride home.", 0);
        assert_eq!(r.message(), "RIDE HOME");
        assert!(!r.hidden());
        r.update(1.0, "Ride home.", 0);
        assert_eq!(r.eased(), 0.0); // resting in place

        // Rests SHOW_SECS total, then rolls back up and hides.
        r.update(SHOW_SECS, "Ride home.", 0);
        r.update(1.0, "Ride home.", 0);
        assert!(r.hidden());

        // A change re-arms with the new directive.
        r.update(0.1, "They know. Hands first.", 0);
        assert_eq!(r.message(), "HANDS FIRST");
        assert!(!r.hidden());
    }

    #[test]
    fn exit_open_without_objective_change_says_go_to_elevator() {
        let mut r = MsgRoller::new();
        r.update(0.1, "Ride home.", 0);
        r.update(10.0, "Ride home.", 0);
        assert!(r.hidden());
        // The legacy all-dead auto-open: exits open, objective untouched.
        r.update(0.1, "Ride home.", 2);
        assert_eq!(r.message(), GO_TO_ELEVATOR);
        assert!(!r.hidden());
        // The count staying up does not re-arm forever.
        r.update(10.0, "Ride home.", 2);
        assert!(r.hidden());
    }

    #[test]
    fn simultaneous_change_and_open_prefers_the_objective() {
        let mut r = MsgRoller::new();
        r.update(0.1, "Ride home.", 0);
        r.update(
            0.1,
            "The smile is off. Ride the EXTRACTION CAR home.",
            1, // exit opened the same tick as the objective flip
        );
        assert_eq!(r.message(), "GET TO THE CAR");
    }
}
