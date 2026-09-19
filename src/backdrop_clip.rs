//! BACKDROP OCCLUSION: where the floor hides the void, do not draw the void.
//!
//! The neon-wave backdrop (`Graphics::backdrop`, opcode 24) is an opaque
//! full-screen quad drawn FIRST, and the floor — opaque tiles filling the
//! level rect `(0,0)..(width,height)` — then paints over most of it. On a
//! fill-bound GPU that hidden part is pure waste (a full-screen layer is
//! 1.8 ms of a 16.7 ms frame on a 2018 MacBook Air, `?gpuprobe`), and unlike
//! a shader trick, NOT drawing fragments cannot cost anything.
//!
//! So the backdrop op carries an EXCLUSION RECT in screen space and
//! renderer.js draws the backdrop as up to four strips around it. This module
//! computes that rect: the axis-aligned screen rectangle guaranteed to lie
//! INSIDE the floor's on-screen image. The floor is a world-space rectangle
//! under the camera `screen = centre + R(roll) * (world - focus) * zoom`, i.e.
//! a slightly ROTATED rectangle on screen (the sway roll), so the rect is
//! built from the rotated corners and then inset: by a safety pixel, plus —
//! in the `?pixel=N` world — the art texel the floor's edge is quantized to.
//!
//! Pure math, host-tested; `Camera::floor_occlusion` is the wasm-side caller.

/// The camera terms of `screen = centre + R(roll) * (world - focus) * zoom`.
#[derive(Clone, Copy, Debug)]
pub struct View {
    /// Screen position of the focus point (canvas centre + sway drift).
    pub centre: (f32, f32),
    pub roll: f32,
    pub zoom: f32,
    /// The world point at `centre`.
    pub focus: (f32, f32),
    /// Canvas size in the units of `centre` (CSS px).
    pub screen: (f32, f32),
}

impl View {
    pub fn world_to_screen(&self, wx: f32, wy: f32) -> (f32, f32) {
        let (sn, cs) = self.roll.sin_cos();
        let dx = (wx - self.focus.0) * self.zoom;
        let dy = (wy - self.focus.1) * self.zoom;
        (
            self.centre.0 + dx * cs - dy * sn,
            self.centre.1 + dx * sn + dy * cs,
        )
    }

    pub fn screen_to_world(&self, sx: f32, sy: f32) -> (f32, f32) {
        let (sn, cs) = (-self.roll).sin_cos();
        let px = sx - self.centre.0;
        let py = sy - self.centre.1;
        (
            self.focus.0 + (px * cs - py * sn) / self.zoom,
            self.focus.1 + (px * sn + py * cs) / self.zoom,
        )
    }
}

/// Smallest exclusion worth a strip layout (CSS px per side).
const MIN_SIDE: f32 = 16.0;

/// The screen rect `[x, y, w, h]` that lies entirely inside the on-screen
/// image of the world rect `(0,0)..(world_w, world_h)`, shrunk by `inset`
/// screen px on every side and clamped to the screen; `None` when the floor
/// is off-screen, the roll is not small, or what is left is not worth it.
///
/// For a rectangle rotated by a small angle, the axis-aligned rect between
/// the innermost x of its left / right edges and the innermost y of its top /
/// bottom edges is inside it: within that y-range every point of the left
/// edge has `x <= left`, and likewise for the other three edges.
pub fn floor_occlusion(view: &View, world_w: f32, world_h: f32, inset: f32) -> Option<[f32; 4]> {
    // (`zoom <= 0.0 || is_nan`: a NaN zoom must exclude nothing either)
    if view.zoom <= 0.0 || view.zoom.is_nan() || view.roll.abs() > 0.5 {
        return None;
    }
    let tl = view.world_to_screen(0.0, 0.0);
    let tr = view.world_to_screen(world_w, 0.0);
    let br = view.world_to_screen(world_w, world_h);
    let bl = view.world_to_screen(0.0, world_h);
    let left = tl.0.max(bl.0) + inset;
    let right = tr.0.min(br.0) - inset;
    let top = tl.1.max(tr.1) + inset;
    let bottom = bl.1.min(br.1) - inset;
    let x0 = left.max(0.0);
    let y0 = top.max(0.0);
    let x1 = right.min(view.screen.0);
    let y1 = bottom.min(view.screen.1);
    if x1 - x0 < MIN_SIDE || y1 - y0 < MIN_SIDE {
        return None;
    }
    Some([x0, y0, x1 - x0, y1 - y0])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(focus: (f32, f32), roll: f32, zoom: f32) -> View {
        View {
            centre: (722.5, 259.0),
            roll,
            zoom,
            focus,
            screen: (1440.0, 523.0),
        }
    }

    /// Every point of the exclusion (corners, edges, interior) maps back
    /// INSIDE the floor, with the inset to spare — for a sweep of rolls, zooms
    /// and camera positions incl. the floor's corners and edges.
    #[test]
    fn the_exclusion_always_lies_inside_the_floor() {
        let (ww, wh) = (1600.0_f32, 1200.0_f32);
        let mut checked = 0;
        for &roll in &[-0.0062_f32, -0.003, 0.0, 0.002, 0.0061, 0.05, -0.2] {
            for &zoom in &[0.6_f32, 1.0, 1.35, 2.0] {
                for &focus in &[
                    (0.0_f32, 0.0_f32),
                    (800.0, 600.0),
                    (1600.0, 1200.0),
                    (40.0, 1180.0),
                    (1590.0, 300.0),
                    (-200.0, 600.0),
                ] {
                    let v = view(focus, roll, zoom);
                    let Some([x, y, w, h]) = floor_occlusion(&v, ww, wh, 1.0) else {
                        continue;
                    };
                    assert!(x >= 0.0 && y >= 0.0 && x + w <= 1440.0 && y + h <= 523.0);
                    for i in 0..=8 {
                        for j in 0..=8 {
                            let sx = x + w * i as f32 / 8.0;
                            let sy = y + h * j as f32 / 8.0;
                            let (wx, wy) = v.screen_to_world(sx, sy);
                            // >= half the 1-px inset survives in world units
                            let slack = 0.5 / zoom - 1e-3;
                            assert!(
                                wx >= slack && wy >= slack && wx <= ww - slack && wy <= wh - slack,
                                "roll {roll} zoom {zoom} focus {focus:?}: screen ({sx},{sy}) -> world ({wx},{wy})"
                            );
                            checked += 1;
                        }
                    }
                }
            }
        }
        assert!(
            checked > 5000,
            "the sweep must actually exercise exclusions ({checked})"
        );
    }

    /// Mid-floor with no roll the whole screen is excluded (minus the clamp):
    /// the backdrop then costs nothing at all.
    #[test]
    fn a_screen_full_of_floor_excludes_the_whole_screen() {
        let v = view((800.0, 600.0), 0.0, 1.0);
        assert_eq!(
            floor_occlusion(&v, 1600.0, 1200.0, 1.0),
            Some([0.0, 0.0, 1440.0, 523.0])
        );
    }

    /// At the floor's corner only the floor's quadrant is excluded, inset.
    #[test]
    fn at_a_corner_only_the_floor_quadrant_is_excluded() {
        let v = view((0.0, 0.0), 0.0, 1.0);
        let [x, y, w, h] = floor_occlusion(&v, 1600.0, 1200.0, 2.0).unwrap();
        assert_eq!((x, y), (724.5, 261.0));
        assert_eq!((x + w, y + h), (1440.0, 523.0));
    }

    /// No floor on screen, a sliver, a big roll, a degenerate zoom: no rect.
    #[test]
    fn degenerate_views_exclude_nothing() {
        assert_eq!(
            floor_occlusion(&view((-5000.0, 0.0), 0.0, 1.0), 1600.0, 1200.0, 1.0),
            None
        );
        assert_eq!(
            floor_occlusion(&view((-715.0, 600.0), 0.0, 1.0), 1600.0, 1200.0, 1.0),
            None
        );
        assert_eq!(
            floor_occlusion(&view((800.0, 600.0), 0.9, 1.0), 1600.0, 1200.0, 1.0),
            None
        );
        assert_eq!(
            floor_occlusion(&view((800.0, 600.0), 0.0, 0.0), 1600.0, 1200.0, 1.0),
            None
        );
    }

    /// A bigger inset (the `?pixel=N` art texel) only ever shrinks the rect.
    #[test]
    fn the_inset_shrinks_the_rect_monotonically() {
        let v = view((100.0, 1100.0), 0.0061, 1.35);
        let a = floor_occlusion(&v, 1600.0, 1200.0, 1.0).unwrap();
        let b = floor_occlusion(&v, 1600.0, 1200.0, 9.0).unwrap();
        assert!(b[0] >= a[0] && b[1] >= a[1]);
        assert!(b[0] + b[2] <= a[0] + a[2] && b[1] + b[3] <= a[1] + a[3]);
    }
}
