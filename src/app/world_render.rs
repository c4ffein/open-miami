//! The app side of `render::world::render_world`: own the frame's two state
//! changes (kill-flash countdown, spark expiry), sample the input, build
//! the read-only `WorldView`.

use super::*;
use crate::render::world::WorldView;

impl GameState {
    /// Draw the world layer. `dt` only drives the kill-flash decay, so the
    /// pause screen re-draws the frozen world behind its modal with 0.
    pub(crate) fn render_world(&mut self, graphics: &Graphics, dt: f32, accent: (u8, u8, u8)) {
        let now = self.last_time as f32 / 1000.0;
        // A flash frame tints the floor with the value AFTER this decay —
        // down to a last fully-faded one.
        let kill_flash = (self.kill_flash > 0.0).then(|| {
            self.kill_flash = (self.kill_flash - dt).max(0.0);
            self.kill_flash
        });
        self.sparks.expire(now);
        let view = WorldView {
            world: &self.world,
            level: &self.level,
            camera: &self.camera,
            sparks: &self.sparks,
            props: floor_def(self.selected_level).props,
            gate_anchor: self.scenario.as_ref().and_then(|sc| sc.gate_anchor()),
            now,
            pixel_world: self.pixel_world,
            kill_flash,
            show_infos: self.show_infos,
            floor_static_key: self.floor_static_key,
            accent,
            player_firing: input::is_mouse_button_down(input::mouse_buttons::LEFT),
        };
        crate::render::world::render_world(graphics, &view);
    }
}
