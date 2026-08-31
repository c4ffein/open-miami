// The sliding bottom-left AMMO BOX: the slide state machine + the text
// choice. Pure host-testable logic — `render::render_ammo_box` does the
// drawing, `GameState` owns one `AmmoSlide` and feeds it `dt` + whether a
// live gun is held.

use crate::components::WeaponType;

/// Seconds for a full slide (shown <-> hidden).
pub const SLIDE_SECS: f32 = 0.3;

/// Slide state for the ammo box. `offset` runs 0 (box in place) .. 1 (fully
/// below the bottom screen edge) and moves LINEARLY toward `target`;
/// `eased()` shapes it (smoothstep) for rendering so the motion accelerates
/// out and settles in.
pub struct AmmoSlide {
    /// Slide progress: 0 = shown in place, 1 = fully off-screen below.
    offset: f32,
    /// Where `offset` is headed (0 shown, 1 hidden).
    target: f32,
    /// Whether a live gun was held last frame (melee/fist = not armed).
    armed: bool,
}

/// A "live gun" for the box: something that actually holds rounds. Fists
/// (`None`) and melee weapons never show ammo.
pub fn gun_held(weapon: Option<WeaponType>) -> bool {
    weapon.is_some_and(|t| !t.is_melee())
}

/// The one line the box shows: `12/12 RNDS` for a gun, `NO GUN` otherwise
/// (the text the box slides out displaying).
pub fn ammo_box_text(weapon: Option<WeaponType>, ammo: i32) -> String {
    match weapon {
        Some(t) if !t.is_melee() => format!("{}/{} RNDS", ammo, t.magazine()),
        _ => "NO GUN".to_string(),
    }
}

impl AmmoSlide {
    /// Fresh state, hidden with no animation pending (the no-gun floor
    /// start).
    pub fn new() -> Self {
        Self {
            offset: 1.0,
            target: 1.0,
            armed: false,
        }
    }

    /// Snap instantly to the state for `gun_held` — floor load / checkpoint
    /// restore: no slide animation on load.
    pub fn snap(&mut self, gun_held: bool) {
        self.armed = gun_held;
        self.target = if gun_held { 0.0 } else { 1.0 };
        self.offset = self.target;
    }

    /// Advance the slide. An armed-state flip retargets immediately (the
    /// TEXT is the caller's business and always current); the offset then
    /// moves linearly and SNAPS onto the target at the end.
    pub fn update(&mut self, dt: f32, gun_held: bool) {
        if gun_held != self.armed {
            self.armed = gun_held;
            self.target = if gun_held { 0.0 } else { 1.0 };
        }
        let step = dt / SLIDE_SECS;
        if self.offset < self.target {
            self.offset = (self.offset + step).min(self.target);
        } else {
            self.offset = (self.offset - step).max(self.target);
        }
    }

    /// Smoothstep-eased slide for rendering: 0 = in place, 1 = fully out.
    pub fn eased(&self) -> f32 {
        let t = self.offset.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// Fully slid out — the renderer skips the box entirely.
    pub fn hidden(&self) -> bool {
        self.offset >= 1.0
    }
}

impl Default for AmmoSlide {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slides_toward_target_and_snaps_at_end() {
        let mut s = AmmoSlide::new();
        assert!(s.hidden()); // no-gun start: already out, no animation

        // Grab a gun: the box rises over SLIDE_SECS, then snaps to 0.
        s.update(0.1, true);
        assert!(s.eased() < 1.0 && !s.hidden());
        let mid = s.eased();
        s.update(0.1, true);
        assert!(s.eased() < mid);
        s.update(0.2, true); // overshoots the remaining time -> snapped
        assert_eq!(s.eased(), 0.0);
        assert!(!s.hidden());

        // Throw it: slides back down and snaps hidden.
        s.update(0.15, false);
        assert!(!s.hidden() && s.eased() > 0.0);
        s.update(1.0, false);
        assert!(s.hidden());
        assert_eq!(s.eased(), 1.0);
    }

    #[test]
    fn snap_is_instant_both_ways() {
        let mut s = AmmoSlide::new();
        s.snap(true);
        assert_eq!(s.eased(), 0.0); // floor starts armed: shown, no slide-in
        s.snap(false);
        assert!(s.hidden()); // and back
    }

    #[test]
    fn text_choice_per_weapon_state() {
        assert_eq!(ammo_box_text(Some(WeaponType::Pistol), 12), "12/12 RNDS");
        assert_eq!(ammo_box_text(Some(WeaponType::Shotgun), 3), "3/6 RNDS");
        assert_eq!(ammo_box_text(Some(WeaponType::Melee), 999), "NO GUN");
        assert_eq!(ammo_box_text(None, 0), "NO GUN");
        assert!(gun_held(Some(WeaponType::MachineGun)));
        assert!(!gun_held(Some(WeaponType::Melee)));
        assert!(!gun_held(None));
    }
}
