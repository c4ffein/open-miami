//! The SPRITES > PROPS page: the animated prop library, per-prop art-pixel
//! size, layer eye / solo / BEFORE-AFTER toggles, SAVE.

use super::*;

/// The `?viz` PROPS page's editable state of one prop: its art-pixel
/// size, which layers the preview shows and each layer's pixel mode
/// (initialised from the saved `PROP_SETTINGS`, written back by SAVE).
#[derive(Clone, Copy)]
pub(crate) struct PropViz {
    pub(crate) px: u32,
    /// Bit i = layer i shown in the big preview (eye / solo).
    pub(crate) visible: u32,
    pub(crate) modes: [PixelMode; MAX_LAYERS],
}

impl GameState {
    /// The PROPS page of the SPRITES tab: the prop library
    /// (`crate::props`) as a live-animated grid — one page per FAMILY
    /// (DATACENTER / OUTDOOR / LOBBY, the buttons at the right of the
    /// header) — with the selected prop enlarged on the right — its
    /// layers listed underneath (eye = hide, S = solo, BEFORE/AFTER = the
    /// layer's pixel mode) — plus the per-prop PIXEL size, the GRID
    /// overlay and SAVE (writes `props/props.json`; then `make
    /// gen-props`).
    pub(crate) fn draw_viz_props(&mut self, graphics: &Graphics, mouse: Vec2, click: bool) {
        let time = (self.last_time / 1000.0) as f32;
        let (w, h) = (graphics.width(), graphics.height());
        let mut sel = self.viz_prop_selected.min(PROP_COUNT - 1);
        let mut fam = self.viz_prop_family.min(PROP_FAMILIES.len() - 1);
        graphics.draw_text(
            "PROP LIBRARY — primitive-drawn, animated, layered; one page per family. Click \
             a tile to enlarge. SAVE writes props/props.json, then run `make gen-props`.",
            Vec2::new(40.0, 138.0),
            16.0,
            Color::GRAY,
        );

        // Header row, right of the page buttons: the SELECTED prop's
        // art-pixel size (design units of its 100x100 box; 1 = off), the
        // GRID overlay toggle for the preview, SAVE.
        let cx = 40.0 + 2.0 * 168.0 + 40.0;
        graphics.draw_text("PIXEL", Vec2::new(cx, 76.0 + 19.0 + 6.0), 18.0, Color::GRAY);
        if viz_button(graphics, mouse, cx + 60.0, 76.0, 38.0, 38.0, "-", false) && click {
            let p = &mut self.viz_props[sel];
            p.px = p.px.saturating_sub(1).max(1);
        }
        let px_label = if self.viz_props[sel].px <= 1 {
            "OFF".to_string()
        } else {
            format!("{}", self.viz_props[sel].px)
        };
        graphics.draw_text(
            &px_label,
            Vec2::new(cx + 108.0, 76.0 + 19.0 + 6.0),
            18.0,
            Color::WHITE,
        );
        if viz_button(graphics, mouse, cx + 148.0, 76.0, 38.0, 38.0, "+", false) && click {
            let p = &mut self.viz_props[sel];
            p.px = (p.px + 1).min(MAX_PX);
        }
        if viz_button(
            graphics,
            mouse,
            cx + 204.0,
            76.0,
            84.0,
            38.0,
            "GRID",
            self.viz_pixel_grid,
        ) && click
        {
            self.viz_pixel_grid = !self.viz_pixel_grid;
        }
        if viz_button(graphics, mouse, cx + 304.0, 76.0, 84.0, 38.0, "SAVE", false) && click {
            let entries: Vec<(u32, [PixelMode; MAX_LAYERS])> =
                self.viz_props.iter().map(|p| (p.px, p.modes)).collect();
            viz_save_props(&settings_json(&entries));
        }
        // The family pages (the tile grid shows one family at a time;
        // switching pages selects that family's first prop).
        for (f, &(name, first)) in PROP_FAMILIES.iter().enumerate() {
            let fx = cx + 414.0 + f as f32 * 120.0;
            if viz_button(graphics, mouse, fx, 76.0, 112.0, 38.0, name, fam == f)
                && click
                && fam != f
            {
                self.viz_prop_family = f;
                self.viz_prop_selected = first;
                fam = f;
                sel = first;
            }
        }

        let cols = 4usize;
        let rows = largest_family().div_ceil(cols);
        let (x0, y0) = (40.0f32, 152.0f32);
        let tile_w = 150.0f32;
        let tile_h = ((h - y0 - 16.0) / rows as f32).clamp(64.0, 110.0);
        for (slot, i) in family_range(fam).enumerate() {
            let name = PROP_NAMES[i];
            let bx = x0 + (slot % cols) as f32 * tile_w;
            let by = y0 + (slot / cols) as f32 * tile_h;
            let (bw, bh) = (tile_w - 6.0, tile_h - 6.0);
            let over = mouse.x >= bx && mouse.x <= bx + bw && mouse.y >= by && mouse.y <= by + bh;
            let selected = sel == i;
            let bg = if selected {
                Color::new(1.0, 0.09, 0.26, 0.30)
            } else if over {
                Color::new(0.28, 0.22, 0.33, 1.0)
            } else {
                Color::new(0.13, 0.09, 0.17, 1.0)
            };
            let border = if selected {
                Color::new(1.0, 0.09, 0.26, 1.0)
            } else {
                Color::new(0.4, 0.3, 0.45, 1.0)
            };
            graphics.draw_rectangle(Vec2::new(bx, by), bw, bh, bg);
            graphics.draw_rectangle_lines(Vec2::new(bx, by), bw, bh, 1.5, border);
            // Every tile at its own prop's saved / edited pixel size and
            // layer modes (all layers: hide / solo are preview-only).
            let pv = self.viz_props[i];
            let opts = PropDrawOpts {
                visible: u32::MAX,
                modes: pv.modes,
            };
            draw_prop_ex(
                graphics,
                i,
                Vec2::new(bx + bw / 2.0, by + bh / 2.0 - 6.0),
                snap_size(bh - 30.0, pv.px),
                time,
                pv.px,
                &opts,
            );
            graphics.draw_text(
                name,
                Vec2::new(bx + 6.0, by + bh - 5.0),
                12.0,
                if selected { Color::WHITE } else { Color::GRAY },
            );
            if over && click {
                self.viz_prop_selected = i;
            }
        }

        // Big live preview of the selected prop, in place of the iframe,
        // with its LAYERS list along the bottom of the panel.
        let px = x0 + cols as f32 * tile_w + 20.0;
        let pw = (w - px - 40.0).max(140.0);
        let ph = rows as f32 * tile_h - 6.0;
        graphics.draw_rectangle(Vec2::new(px, y0), pw, ph, Color::new(0.07, 0.05, 0.10, 1.0));
        graphics.draw_rectangle_lines(
            Vec2::new(px, y0),
            pw,
            ph,
            1.5,
            Color::new(0.4, 0.3, 0.45, 1.0),
        );
        let layers = prop_layers(sel);
        let row_h = 24.0;
        let list_h = 30.0 + layers.len() as f32 * row_h + 8.0;
        let area_top = y0 + 56.0;
        let area_h = (ph - 56.0 - list_h).max(60.0);
        let pv = self.viz_props[sel];
        // Integer texel -> device pixel magnification (see `snap_size`).
        let size = snap_size((pw.min(area_h) * 0.8).max(40.0), pv.px);
        let center = Vec2::new(px + pw / 2.0, area_top + area_h / 2.0);
        let opts = PropDrawOpts {
            visible: pv.visible,
            modes: pv.modes,
        };
        draw_prop_ex(graphics, sel, center, size, time, pv.px, &opts);
        // The art-pixel grid of the prop's own frame (the grid every
        // fixed layer sits on), anchored to the prop centre; readable
        // from 3 screen px per art pixel up.
        let cell = pv.px as f32 * size / 100.0;
        if self.viz_pixel_grid && pv.px >= 2 && cell >= 3.0 {
            let gc = Color::new(1.0, 1.0, 1.0, 0.16);
            let half = size * 0.55;
            let n = (half / cell).ceil() as i32;
            for k in -n..=n {
                let o = k as f32 * cell;
                graphics.draw_line(
                    Vec2::new(center.x + o, center.y - half),
                    Vec2::new(center.x + o, center.y + half),
                    1.0,
                    gc,
                );
                graphics.draw_line(
                    Vec2::new(center.x - half, center.y + o),
                    Vec2::new(center.x + half, center.y + o),
                    1.0,
                    gc,
                );
            }
        }
        graphics.draw_text(
            PROP_NAMES[sel],
            Vec2::new(px + 16.0, y0 + 28.0),
            22.0,
            Color::WHITE,
        );
        graphics.draw_text(
            &format!(
                "prop {:02} / {}  ·  {}  ·  {} layers  ·  pixel {}",
                sel,
                PROP_COUNT,
                PROP_FAMILIES[prop_family(sel)].0,
                layers.len(),
                if pv.px <= 1 {
                    "off".to_string()
                } else {
                    format!("{} ({} art px across)", pv.px, 100 / pv.px)
                }
            ),
            Vec2::new(px + 16.0, y0 + 48.0),
            14.0,
            Color::GRAY,
        );

        // LAYERS: one row per layer — eye (hide), S (solo), name, its
        // rotation, and the BEFORE / AFTER pixel-mode toggle.
        let ly = y0 + ph - list_h;
        graphics.draw_line(
            Vec2::new(px + 1.0, ly),
            Vec2::new(px + pw - 1.0, ly),
            1.0,
            Color::new(0.4, 0.3, 0.45, 1.0),
        );
        graphics.draw_text(
            "LAYERS, bottom to top   o = show/hide   S = solo   BEFORE / AFTER = pixelate before / after its rotation",
            Vec2::new(px + 12.0, ly + 20.0),
            13.0,
            Color::GRAY,
        );
        let all_mask = if layers.len() >= 32 {
            u32::MAX
        } else {
            (1u32 << layers.len()) - 1
        };
        for (i, l) in layers.iter().enumerate() {
            let ry = ly + 30.0 + i as f32 * row_h;
            let bit = 1u32 << i;
            let shown = pv.visible & bit != 0;
            let solo = pv.visible & all_mask == bit;
            let rx = px + 12.0;
            if viz_small_button(graphics, mouse, rx, ry, 26.0, 20.0, "o", shown) && click {
                self.viz_props[sel].visible ^= bit;
            }
            if viz_small_button(graphics, mouse, rx + 32.0, ry, 26.0, 20.0, "S", solo) && click {
                self.viz_props[sel].visible = if solo { u32::MAX } else { bit };
            }
            graphics.draw_text(
                l.name,
                Vec2::new(rx + 70.0, ry + 15.0),
                16.0,
                if shown { Color::WHITE } else { Color::GRAY },
            );
            graphics.draw_text(
                &l.rot.label(),
                Vec2::new(rx + 170.0, ry + 15.0),
                13.0,
                Color::GRAY,
            );
            let mode = pv.modes[i];
            let bx = px + pw - 12.0 - 84.0;
            if viz_small_button(
                graphics,
                mouse,
                bx,
                ry,
                84.0,
                20.0,
                match mode {
                    PixelMode::Before => "BEFORE",
                    PixelMode::After => "AFTER",
                },
                mode == PixelMode::After,
            ) && click
            {
                self.viz_props[sel].modes[i] = mode.toggled();
            }
        }
    }
}
