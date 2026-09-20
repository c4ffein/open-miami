use crate::graphics::Graphics;
use crate::math::Vec2;

// The pure culling predicate + rect live in src/math.rs so their tests run on
// the host (this module is wasm-only); re-exported here for the renderers.
pub use crate::math::{rect_visible, ViewCull, CULL_MARGIN};

/// Default zoom: how many screen pixels one world unit covers. >1 pulls the
/// camera in closer to the characters.
pub const DEFAULT_ZOOM: f32 = 1.6;

/// The viewport the game was tuned at: `set_viewport` scales the zoom from
/// it by the geometric mean of the two axes (~constant visible AREA whatever
/// the window size or aspect — a 2:1 window trades height of view for width
/// instead of simply seeing more), clamped so the graphics never shrink
/// below readability on a small window nor balloon on a huge one. Inside the
/// clamp band, what you can see stays fair; at the band's edges, legibility
/// wins and the window sees less (or more) of the world instead.
pub const REF_VIEW_W: f32 = 960.0;
pub const REF_VIEW_H: f32 = 720.0;
pub const ZOOM_SCALE_MIN: f32 = 0.75;
pub const ZOOM_SCALE_MAX: f32 = 1.5;

/// How far (in screen px, before zoom) the view may be pushed toward the mouse
/// while Shift is held — a look-ahead so you can peek down a corridor.
pub const LOOK_MAX_PX: f32 = 260.0;
/// Fraction of the mouse's offset from screen centre that becomes look-ahead.
pub const LOOK_FACTOR: f32 = 0.55;
/// Look-ahead easing rate (per second); higher snaps faster.
pub const LOOK_EASE: f32 = 9.0;

/// Camera sway: a barely-there roll of the whole view, like a slow breath /
/// slightly drunk hand-held feel. Amplitude in radians (0.35°) and rate in Hz.
pub const SWAY_ROLL_RAD: f32 = 0.0061;
pub const SWAY_ROLL_HZ: f32 = 0.11;
/// A matching sub-pixel drift so the roll doesn't read as pure rotation.
pub const SWAY_DRIFT_PX: f32 = 2.5;
pub const SWAY_DRIFT_HZ: f32 = 0.07;

pub struct Camera {
    /// World point the camera follows (the player).
    pub target: Vec2,
    /// Current smoothed look-ahead offset, in WORLD units, added to `target`.
    pub look: Vec2,
    /// Scenario `look_at` offset (world units): pulls the focus toward a
    /// point of interest by its weight (see `set_cinematic`).
    pub cine: Vec2,
    /// World-units -> screen-px scale.
    pub zoom: f32,
    /// Current sway (set by `update_sway`): screen-space drift + roll.
    sway_dx: f32,
    sway_dy: f32,
    sway_roll: f32,
    canvas_width: f32,
    canvas_height: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            target: Vec2::zero(),
            look: Vec2::zero(),
            cine: Vec2::zero(),
            zoom: DEFAULT_ZOOM,
            sway_dx: 0.0,
            sway_dy: 0.0,
            sway_roll: 0.0,
            canvas_width: 960.0,
            canvas_height: 720.0,
        }
    }
}

impl Camera {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record the real canvas size so apply()/screen_to_world()/visible_bounds()
    /// all agree even when the window isn't 960x720, and derive the zoom from
    /// it (see `REF_VIEW_W`): bigger windows draw the world bigger rather than
    /// revealing arbitrarily much of it, until legibility caps out.
    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.canvas_width = width;
        self.canvas_height = height;
        let area_scale = ((width * height) / (REF_VIEW_W * REF_VIEW_H)).sqrt();
        self.zoom = DEFAULT_ZOOM * area_scale.clamp(ZOOM_SCALE_MIN, ZOOM_SCALE_MAX);
    }

    pub fn follow_player(&mut self, player_pos: Vec2) {
        self.target = player_pos;
    }

    /// Ease the look-ahead toward `mouse_screen` (when `active`, i.e. Shift is
    /// held) or back to centre. Uses the mouse's SCREEN offset from centre so it
    /// doesn't feed back through its own transform.
    pub fn update_look(&mut self, mouse_screen: Vec2, active: bool, dt: f32) {
        let goal = if active {
            let cx = self.canvas_width / 2.0;
            let cy = self.canvas_height / 2.0;
            let mut ox = (mouse_screen.x - cx) * LOOK_FACTOR;
            let mut oy = (mouse_screen.y - cy) * LOOK_FACTOR;
            let len = (ox * ox + oy * oy).sqrt();
            if len > LOOK_MAX_PX {
                ox *= LOOK_MAX_PX / len;
                oy *= LOOK_MAX_PX / len;
            }
            // screen px -> world units
            Vec2::new(ox / self.zoom, oy / self.zoom)
        } else {
            Vec2::zero()
        };
        let k = (dt * LOOK_EASE).clamp(0.0, 1.0);
        self.look.x += (goal.x - self.look.x) * k;
        self.look.y += (goal.y - self.look.y) * k;
    }

    /// Scenario `look_at`: `Some((point, weight))` moves the focus toward the
    /// world `point` by `weight` (0 = on the player, 1 = on the point); the
    /// scenario eases the weight in and out. `None` clears it.
    pub fn set_cinematic(&mut self, focus: Option<(Vec2, f32)>) {
        self.cine = match focus {
            Some((p, w)) => {
                let w = w.clamp(0.0, 1.0);
                Vec2::new((p.x - self.target.x) * w, (p.y - self.target.y) * w)
            }
            None => Vec2::zero(),
        };
    }

    /// The world point that sits at the centre of the screen.
    pub fn focus(&self) -> Vec2 {
        Vec2::new(
            self.target.x + self.look.x + self.cine.x,
            self.target.y + self.look.y + self.cine.y,
        )
    }

    /// Advance the sway for this frame; `time` in seconds. Called once per
    /// frame before `apply()` so apply/screen_to_world share the same values.
    pub fn update_sway(&mut self, time: f32) {
        let two_pi = std::f32::consts::TAU;
        self.sway_roll = (time * SWAY_ROLL_HZ * two_pi).sin() * SWAY_ROLL_RAD;
        self.sway_dx = (time * SWAY_DRIFT_HZ * two_pi).sin() * SWAY_DRIFT_PX;
        self.sway_dy = (time * SWAY_DRIFT_HZ * two_pi * 1.37 + 1.1).cos() * SWAY_DRIFT_PX;
    }

    pub fn apply(&self, graphics: &Graphics) {
        // screen = centre + drift + R(roll) * (world - focus) * zoom
        let f = self.focus();
        self.apply_composite(graphics);
        graphics.translate(-f.x, -f.y);
    }

    /// The COMPOSITE half of `apply()`: centre + drift + roll + zoom, without
    /// the `-focus` translation. This is the transform the pixelated world
    /// places its finished art-res image under (`render_world`): the group
    /// rasterizes on a world-anchored grid and this rigid transform is what
    /// moves / sways / rotates the whole pixel image at native resolution —
    /// the vibe's "Before"-mode rotation at the composite quad. After it,
    /// local units are world units (the zoom is inside), and `apply()` ==
    /// `apply_composite()` + `translate(-focus)`. Balanced by `reset()`.
    pub fn apply_composite(&self, graphics: &Graphics) {
        graphics.save();
        graphics.translate(
            self.canvas_width / 2.0 + self.sway_dx,
            self.canvas_height / 2.0 + self.sway_dy,
        );
        graphics.rotate(self.sway_roll);
        graphics.scale(self.zoom, self.zoom);
    }

    pub fn reset(&self, graphics: &Graphics) {
        graphics.restore();
    }

    /// World-space bounds currently visible on screen. Matches `apply()`.
    pub fn visible_bounds(&self, screen_width: f32, screen_height: f32) -> (Vec2, Vec2) {
        let f = self.focus();
        let half_w = screen_width / (2.0 * self.zoom);
        let half_h = screen_height / (2.0 * self.zoom);
        (
            Vec2::new(f.x - half_w, f.y - half_h),
            Vec2::new(f.x + half_w, f.y + half_h),
        )
    }

    /// The visible bounds inflated by `CULL_MARGIN`, packaged for the world
    /// renderers to skip off-screen sprites (see `ViewCull`).
    pub fn view_cull(&self, screen_width: f32, screen_height: f32) -> ViewCull {
        let (min, max) = self.visible_bounds(screen_width, screen_height);
        ViewCull::new(min, max)
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// The screen rect `[x, y, w, h]` the floor `(0,0)..(world_w, world_h)`
    /// is guaranteed to cover under this frame's `apply()` transform (sway
    /// included), shrunk by `inset` px — what the backdrop need not draw
    /// (see [`crate::backdrop_clip`]).
    pub fn floor_occlusion(&self, world_w: f32, world_h: f32, inset: f32) -> Option<[f32; 4]> {
        let f = self.focus();
        let view = crate::backdrop_clip::View {
            centre: (
                self.canvas_width / 2.0 + self.sway_dx,
                self.canvas_height / 2.0 + self.sway_dy,
            ),
            roll: self.sway_roll,
            zoom: self.zoom,
            focus: (f.x, f.y),
            screen: (self.canvas_width, self.canvas_height),
        };
        crate::backdrop_clip::floor_occlusion(&view, world_w, world_h, inset)
    }

    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2 {
        // Exact inverse of apply(): undo drift, roll, zoom, then re-add focus.
        let f = self.focus();
        let px = screen_pos.x - (self.canvas_width / 2.0 + self.sway_dx);
        let py = screen_pos.y - (self.canvas_height / 2.0 + self.sway_dy);
        let (sn, cs) = (-self.sway_roll).sin_cos();
        let rx = px * cs - py * sn;
        let ry = px * sn + py * cs;
        Vec2::new(f.x + rx / self.zoom, f.y + ry / self.zoom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::stream::{check, final_transform};

    /// A camera mid-game: off-origin, looking ahead, cinematic pull, swaying.
    fn busy_camera(w: f32, h: f32) -> Camera {
        let mut cam = Camera::new();
        cam.set_viewport(w, h);
        cam.follow_player(Vec2::new(812.0, 431.0));
        cam.update_look(Vec2::new(w * 0.9, h * 0.2), true, 0.05);
        cam.set_cinematic(Some((Vec2::new(300.0, 900.0), 0.4)));
        cam.update_sway(3.7);
        cam
    }

    /// The transform `apply()` records, as renderer.js would build it.
    fn recorded(cam: &Camera, w: f32, h: f32) -> crate::graphics::stream::Affine {
        let g = Graphics::new_headless(w, h);
        cam.apply(&g);
        let frame = g.take_frame();
        // Unbalanced on purpose (reset() closes it): decode, don't `check`.
        final_transform(&crate::graphics::stream::walk(&frame.cmds).unwrap())
    }

    #[test]
    fn zoom_tracks_the_viewport_area_and_clamps() {
        let mut cam = Camera::new();
        cam.set_viewport(REF_VIEW_W, REF_VIEW_H);
        assert!((cam.zoom() - DEFAULT_ZOOM).abs() < 1e-6);
        cam.set_viewport(200.0, 150.0);
        assert!((cam.zoom() - DEFAULT_ZOOM * ZOOM_SCALE_MIN).abs() < 1e-6);
        cam.set_viewport(5120.0, 2880.0);
        assert!((cam.zoom() - DEFAULT_ZOOM * ZOOM_SCALE_MAX).abs() < 1e-6);
        // Same area, other aspect = same zoom (constant visible area).
        cam.set_viewport(1440.0, 480.0);
        assert!((cam.zoom() - DEFAULT_ZOOM).abs() < 1e-4);
    }

    #[test]
    fn the_focus_lands_on_the_swayed_screen_centre() {
        let (w, h) = (1280.0, 800.0);
        let cam = busy_camera(w, h);
        let f = cam.focus();
        let (sx, sy) = recorded(&cam, w, h).apply(f.x, f.y);
        assert!((sx - w / 2.0).abs() <= SWAY_DRIFT_PX + 1e-3);
        assert!((sy - h / 2.0).abs() <= SWAY_DRIFT_PX + 1e-3);
    }

    #[test]
    fn screen_to_world_inverts_the_recorded_transform() {
        for (w, h) in [(960.0, 720.0), (1280.0, 800.0), (2880.0, 1046.0)] {
            let cam = busy_camera(w, h);
            let m = recorded(&cam, w, h);
            for world in [(0.0, 0.0), (812.0, 431.0), (1500.0, -40.0), (-300.0, 990.0)] {
                let (sx, sy) = m.apply(world.0, world.1);
                let back = cam.screen_to_world(Vec2::new(sx, sy));
                assert!(
                    (back.x - world.0).abs() < 0.05 && (back.y - world.1).abs() < 0.05,
                    "{world:?} -> ({sx}, {sy}) -> {back:?} at {w}x{h}"
                );
            }
        }
    }

    #[test]
    fn apply_is_composite_plus_focus_and_reset_balances_it() {
        let (w, h) = (960.0, 720.0);
        let cam = busy_camera(w, h);
        let g = Graphics::new_headless(w, h);
        cam.apply_composite(&g);
        let f = cam.focus();
        g.translate(-f.x, -f.y);
        let split = g.take_frame();
        cam.apply(&g);
        assert_eq!(g.take_frame().cmds, split.cmds);

        cam.apply(&g);
        cam.reset(&g);
        let frame = g.take_frame();
        check(&frame.cmds, &frame.texts).expect("apply + reset is balanced");
    }

    #[test]
    fn visible_bounds_corners_map_to_the_screen_corners_without_sway() {
        let (w, h) = (1280.0, 800.0);
        let mut cam = Camera::new();
        cam.set_viewport(w, h);
        cam.follow_player(Vec2::new(500.0, 500.0));
        let m = recorded(&cam, w, h);
        let (min, max) = cam.visible_bounds(w, h);
        let (x0, y0) = m.apply(min.x, min.y);
        let (x1, y1) = m.apply(max.x, max.y);
        assert!(x0.abs() < 1e-2 && y0.abs() < 1e-2, "({x0}, {y0})");
        assert!(
            (x1 - w).abs() < 1e-2 && (y1 - h).abs() < 1e-2,
            "({x1}, {y1})"
        );
    }

    #[test]
    fn the_occluded_rect_really_is_floor_under_the_recorded_transform() {
        let (w, h) = (1280.0, 800.0);
        let (fw, fh) = (1600.0, 1200.0);
        let cam = busy_camera(w, h);
        let [x, y, rw, rh] = cam
            .floor_occlusion(fw, fh, 2.0)
            .expect("camera is mid-floor");
        assert!(x >= 0.0 && y >= 0.0 && x + rw <= w && y + rh <= h);
        for (sx, sy) in [(x, y), (x + rw, y), (x, y + rh), (x + rw, y + rh)] {
            let p = cam.screen_to_world(Vec2::new(sx, sy));
            assert!(
                p.x >= 0.0 && p.x <= fw && p.y >= 0.0 && p.y <= fh,
                "screen ({sx}, {sy}) = world {p:?} is off the floor"
            );
        }
    }
}
