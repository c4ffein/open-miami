//! `render_world`: one world frame — backdrop, camera, the `?pixel=N` scenery
//! group, the static floor cache, props, elevators, then the actors.

use super::*;

/// Art-pixel size of the neon-wave void backdrop (`Graphics::backdrop`),
/// in CSS px — chunky and cheap (~1/36th of a native-res shader pass).
pub(crate) const BACKDROP_ART_PX: f32 = 6.0;

/// Kill flash: total duration and number of red/blue strobes.
pub(crate) const KILL_FLASH_SECS: f32 = 0.34;
pub(crate) const KILL_FLASH_STROBES: u32 = 4;

/// A small pixel-art arrow pointing DOWN at `(x, y)` (its tip), built
/// from 3-px cells: a 2-cell shaft over a 6/4/2-cell head, with a dark
/// backing shadow so it reads on any floor.
pub(crate) fn draw_pixel_arrow(graphics: &Graphics, x: f32, y: f32, accent: (u8, u8, u8)) {
    const C: f32 = 3.0;
    let col = Color::new(
        accent.0 as f32 / 255.0,
        accent.1 as f32 / 255.0,
        accent.2 as f32 / 255.0,
        0.95,
    );
    let shadow = Color::new(0.0, 0.0, 0.0, 0.45);
    // (dx cells, dy cells, w cells) rows, y grows toward the tip.
    let rows: [(f32, f32, f32); 7] = [
        (-1.0, -7.0, 2.0),
        (-1.0, -6.0, 2.0),
        (-1.0, -5.0, 2.0),
        (-1.0, -4.0, 2.0),
        (-3.0, -3.0, 6.0),
        (-2.0, -2.0, 4.0),
        (-1.0, -1.0, 2.0),
    ];
    for &(dx, dy, w) in &rows {
        graphics.draw_rectangle(
            Vec2::new(x + dx * C + 1.0, y + dy * C + 1.0),
            w * C,
            C,
            shadow,
        );
    }
    for &(dx, dy, w) in &rows {
        graphics.draw_rectangle(Vec2::new(x + dx * C, y + dy * C), w * C, C, col);
    }
}

impl GameState {
    /// The WORLD layer of a frame — camera transform, floor tiles, walls,
    /// props, elevators, corpses, entities, live robot sprites — exactly
    /// what sits between `camera.apply` and `camera.reset` (wrapped in
    /// the `?pixel=N` group when active). Pure rendering: no input, no
    /// simulation, so the pause screen re-draws the frozen world behind
    /// its modal with `dt = 0` (only the kill-flash decay reads `dt`).
    pub(crate) fn render_world(&mut self, graphics: &Graphics, dt: f32, accent: (u8, u8, u8)) {
        // `?pixel=N` (N in WORLD units per art pixel): THE VIBE's endgame —
        // the SCENERY (floor, walls, props, elevators) rasterizes at art
        // resolution on a WORLD-ANCHORED grid (the group origin snaps to
        // whole art pixels of the world, so the texels never re-phase as
        // the camera pans), and the FINISHED pixel image is what the
        // camera moves: the composite half of the camera (centre + drift
        // + roll + zoom) sits OUTSIDE the group, so the sway rotates /
        // glides the rigid pixel image at native resolution
        // ("Before"-mode rotation at the composite quad — never
        // re-rasterization inside the group, which would crawl). The
        // group is drawn with the SUB-PIXEL composite (no origin snap:
        // the motion glides; sampling stays NEAREST — the aliased edges
        // are the art direction, see CLAUDE.md ## Design) and rendered
        // with a bleed margin so the roll never exposes void at the
        // screen edges.
        //
        // The MOVING actors — robots, boss, bullets, weapons, the gate
        // arrow — draw AFTER the group closes, straight under the camera
        // transform: they are already baked pixel sprites (robot tiles,
        // gun sprites) and per the vibe they must MOVE SMOOTHLY at
        // native resolution, never be re-quantized onto the world grid
        // (inside the group a walking robot hops world-texel by
        // world-texel — the whole scene then FEELS snapped even though
        // the backdrop glides). The HUD stays crisp outside everything.
        // The NEON-WAVE VOID (opcode 24) under everything: drawn FIRST,
        // full-screen in SCREEN space (the transform here is identity —
        // before the camera / the `?pixel=N` scenery group), so it is
        // normal frame content in both flat and pixel modes and behind
        // the kill-flash bypass frames alike. The floor tiles + walls
        // paint over it; only the outside-the-level area keeps it.
        // ~6 CSS px per art pixel: chunky, and ~1/36th the fragment work
        // of a native-res pass (DRIVE economics — one upscaled quad).
        // ... minus the part the floor is about to paint over (the floor
        // is opaque over its whole rect): an exclusion rect, inset by a
        // safety pixel plus — in the `?pixel=N` world — the art texel the
        // floor's on-screen edge is quantized to.
        let (floor_min, floor_max) = self.level.full_bounds();
        let texel = if self.pixel_world >= 2 {
            self.pixel_world as f32 * self.camera.zoom()
        } else {
            0.0
        };
        let occluded = self.camera.floor_occlusion(
            floor_max.x - floor_min.x,
            floor_max.y - floor_min.y,
            2.0 + 2.0 * texel,
        );
        graphics.backdrop(
            graphics.width(),
            graphics.height(),
            self.last_time as f32 / 1000.0,
            BACKDROP_ART_PX,
            occluded,
        );
        let (mut view_min, mut view_max) = self
            .camera
            .visible_bounds(graphics.width(), graphics.height());
        let pixel_on = self.pixel_world >= 2;
        if pixel_on {
            let px = self.pixel_world as f32;
            // Bleed: covers the sway roll's edge excursion, the drift and
            // the origin snap (a handful of texels is plenty at 0.35°).
            let margin = px * 4.0 + 16.0;
            let ox = ((view_min.x - margin) / px).floor() * px;
            let oy = ((view_min.y - margin) / px).floor() * px;
            // Whole-texel group size: the renderer's composite flip is
            // anchored at the integer row count, and a fractional height
            // would make the ceil remainder vary as the camera moves.
            let gw = ((view_max.x + margin - ox) / px).ceil() * px;
            let gh = ((view_max.y + margin - oy) / px).ceil() * px;
            let f = self.camera.focus();
            self.camera.apply_composite(graphics);
            // An extra save so closing the scenery group can return to
            // the PURE composite transform — correct even when an
            // oversized group fell back to pass-through (its END is a
            // no-op and the translates below must unwind regardless).
            graphics.save();
            // Place the group rect relative to the focus BEFORE the
            // group opens: pixel_end lands at local (0, 0), which keeps
            // the renderer's oversized-group pass-through fallback
            // seamless (a skipped BEGIN leaves this transform in force).
            graphics.translate(ox - f.x, oy - f.y);
            graphics.pixel_begin_smooth(px, gw, gh);
            graphics.translate(-ox, -oy);
            // Everything the group can show (bleed included) must be
            // drawn: cull to the group rect, not the visible bounds.
            view_min = Vec2::new(ox, oy);
            view_max = Vec2::new(ox + gw, oy + gh);
        } else {
            self.camera.apply(graphics);
        }
        // View culling for the expensive sprites (live 3D robots / guns /
        // the boss) and the placed props: anything whose footprint lies
        // fully outside these inflated bounds skips its commands.
        let cull = crate::camera::ViewCull::new(view_min, view_max);
        // Kill flash: the floor strobes red / blue / red / blue for a beat.
        let tint = if self.kill_flash > 0.0 {
            self.kill_flash = (self.kill_flash - dt).max(0.0);
            let phase = ((KILL_FLASH_SECS - self.kill_flash) / KILL_FLASH_SECS
                * KILL_FLASH_STROBES as f32) as u32;
            let fade = self.kill_flash / KILL_FLASH_SECS; // 1 -> 0
            Some(if phase.is_multiple_of(2) {
                Color::new(0.85, 0.08, 0.16, 0.55 * fade)
            } else {
                Color::new(0.10, 0.25, 0.95, 0.55 * fade)
            })
        } else {
            None
        };
        // STATIC GEOMETRY CACHE: the tiles + walls do not change between
        // frames, so they are baked once per floor into a persistent
        // renderer-side VBO (world coordinates) and every later frame
        // costs a 2-float STATIC_REF + one draw with the camera applied
        // in the vertex shader (`Graphics::static_layer`, opcodes
        // 21/22/23). The cache works inside the `?pixel=N` world group
        // too: the VBO's world coordinates go through the group's
        // world->texel transform via the same vertex-shader affine, so
        // the pixelated world keeps the command-stream win. Bypassed —
        // plain per-frame draws, exactly the old path — whenever the
        // section would not be frame-invariant:
        //   - kill flash: the floor tiles are tinted per frame;
        //   - debug overlays (I): walls draw their inflated pathfinding
        //     boundaries interleaved with the wall rects.
        // The cached VBO survives those bypass frames (the renderer only
        // evicts on a key change), so the flash / overlay toggling off
        // returns to the cache without a re-record.
        if tint.is_none() && !self.show_infos {
            let (floor_min, floor_max) = self.level.full_bounds();
            let level = &self.level;
            let world = &self.world;
            graphics.static_layer(self.floor_static_key, || {
                // Record the WHOLE floor, not the camera-culled range:
                // the cache must be valid for every camera position (the
                // GPU clips off-screen quads for free).
                level.render(graphics, floor_min, floor_max, None);
                render_walls(world, graphics, false);
            });
        } else {
            self.level.render(graphics, view_min, view_max, tint);

            // Render walls from the world
            render_walls(&self.world, graphics, self.show_infos);
        }

        // Placed props: floor furniture over the tiles / walls, under the
        // actors (decoration only, no collision).
        crate::render::floor_props::render_floor_props(
            graphics,
            floor_def(self.selected_level).props,
            self.last_time as f32 / 1000.0,
            &cull,
        );

        // Elevators (recessed door frames; exits light up when open) and,
        // in debug mode, the scenario trigger zones.
        render_elevators(
            &self.world,
            graphics,
            accent,
            self.last_time as f32 / 1000.0,
        );
        if self.show_infos {
            render_zones_debug(&self.world, graphics);
        }

        // Scenery done. In pixel mode: composite the world image NOW
        // (under the sway transform, sub-pixel smooth), then rebuild the
        // plain camera space (composite + translate(-focus)) for the
        // actors — baked pixel sprites gliding at native resolution over
        // the pixelated scenery, the Hotline-Miami layering.
        if pixel_on {
            graphics.pixel_end(0.0, 0.0);
            graphics.restore(); // back to the pure composite transform
            let f = self.camera.focus();
            graphics.translate(-f.x, -f.y);
        }

        // Downed / dead bots first: the ground weapons (in
        // render_entities below) draw OVER the corpses so they stay easy
        // to spot, while everyone still standing draws over the guns.
        draw_robot_entities(
            &self.world,
            graphics,
            self.last_time as f32 / 1000.0,
            true,
            &cull,
        );

        // Render all entities except the player/rogue bots themselves
        // (bullets, pickups, boss, debug overlays...).
        render_entities(
            &self.world,
            graphics,
            self.show_infos,
            self.last_time as f32 / 1000.0,
            &cull,
        );

        // The upright player and rogues are the live 3D robot sprites,
        // drawn while the camera transform (incl. zoom) is still applied
        // so world-space positions and sizes land correctly.
        draw_robot_entities(
            &self.world,
            graphics,
            self.last_time as f32 / 1000.0,
            false,
            &cull,
        );

        // Electric spark bursts where attacks landed on bots: ACTORS
        // layer content (after the `?pixel=N` scenery group closed, over
        // the robots) — each burst is its own small snapped pixel group,
        // so it stays chunky in flat mode too. Expiry mutates the pool
        // in place; nothing allocates.
        {
            let now = self.last_time as f32 / 1000.0;
            self.sparks.expire(now);
            crate::sparks::render_sparks(&self.sparks, graphics, now, &cull);
        }

        // A pixelated arrow slowly floating over the active tutorial
        // gate's target, so "swing the bar" always has an obvious victim.
        if let Some(anchor) = self.scenario.as_ref().and_then(|sc| sc.gate_anchor()) {
            let t = self.last_time as f32 / 1000.0;
            // Bob in whole 2-px steps: floaty but still pixel-crisp.
            let bob = ((t * 2.2).sin() * 3.0).floor() * 2.0;
            draw_pixel_arrow(graphics, anchor.x, anchor.y - 58.0 + bob, accent);
        }

        // Reset camera for UI rendering (in pixel mode the scenery group
        // was already closed before the actors; both paths sit at
        // composite + translate(-focus) = the camera transform here).
        self.camera.reset(graphics);
    }
}
