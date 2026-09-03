use crate::graphics::Graphics;
use crate::math::{visible_tile_range, Color, Vec2};
use crate::palette;
use crate::scenario::Surface;

pub struct Level {
    width: f32,
    height: f32,
    tile_size: f32,
    /// The floor's ground rendering (`FloorDef::surface`).
    surface: Surface,
}

impl Default for Level {
    fn default() -> Self {
        Self {
            width: 2000.0,
            height: 2000.0,
            tile_size: 25.0,
            surface: Surface::Checker,
        }
    }
}

/// Cheap deterministic per-tile hash in `0..1` (tone variation / detail
/// placement). Position-keyed so the floor is frame-invariant — required by
/// the static geometry cache (`Graphics::static_layer`).
fn tile_noise(x: i32, y: i32, salt: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77) ^ salt;
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h & 0xFFFF) as f32 / 65535.0
}

/// Nudge a palette tone's value by `d` (kept opaque).
fn shade(c: Color, d: f32) -> Color {
    Color::new(c.r + d, c.g + d, c.b + d, 1.0)
}

impl Level {
    pub fn new() -> Self {
        Self::default()
    }

    /// Select the ground rendering (called when a floor loads).
    pub fn set_surface(&mut self, surface: Surface) {
        self.surface = surface;
    }

    /// Match the tile field to the floor's playable rect (`FloorDef::width` /
    /// `height`, called when a floor loads). Tiles are only drawn INSIDE
    /// `(0,0)..(width,height)` — outside it the neon-wave void backdrop
    /// (`Graphics::backdrop`, drawn first each frame) shows through.
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.width = width.max(self.tile_size);
        self.height = height.max(self.tile_size);
    }

    pub fn surface(&self) -> Surface {
        self.surface
    }

    /// The whole floor rect `(0,0)..(width,height)` — pass to [`render`]
    /// (`Self::render`) to draw every tile regardless of the camera (the
    /// static geometry cache records the full floor once; see
    /// `Graphics::static_layer`).
    pub fn full_bounds(&self) -> (Vec2, Vec2) {
        (Vec2::new(0.0, 0.0), Vec2::new(self.width, self.height))
    }

    /// Render the floor. Only tiles overlapping `view_min..view_max`
    /// (world-space camera bounds) are drawn, so cost scales with the screen
    /// size rather than the whole floor; tiles are also CLIPPED to the
    /// floor's rect (`set_size`) — outside the level nothing is drawn and
    /// the void backdrop shows.
    ///
    /// `tint` is an optional colour blended over the floor (used for the kill
    /// flash: the background strobes while walls and actors stay untouched);
    /// its alpha is the blend amount.
    ///
    /// Every surface is plain axis-aligned rectangles (one base rect per tile
    /// plus a few detail rects — insets, seams, stains, cracks, paint
    /// remnants, and a fine hash-scattered micro-speckle: dense aggregate
    /// grains on asphalt, sparse grains + pits on concrete, subtle fibre dots
    /// on the carpet, a rare glint on marble, none on the already-busy
    /// grating), all colours from `src/palette.rs` and every detail feature
    /// at least 3 world units thick so it survives `?pixel=3` art-res
    /// rasterization. Detail placement is position-hashed (`tile_noise`) so
    /// every frame records identically — the whole floor goes through the
    /// static geometry cache.
    pub fn render(&self, graphics: &Graphics, view_min: Vec2, view_max: Vec2, tint: Option<Color>) {
        // ceil: a floor size that is not a multiple of the tile size still
        // gets covered to its edge (the last row/column is clamped below).
        let ts = self.tile_size;
        let tiles_x = (self.width / ts).ceil() as i32;
        let tiles_y = (self.height / ts).ceil() as i32;

        let range = visible_tile_range(view_min, view_max, ts, tiles_x, tiles_y);

        let mix = |c: Color| -> Color {
            match tint {
                Some(t) => Color::new(
                    c.r + (t.r - c.r) * t.a,
                    c.g + (t.g - c.g) * t.a,
                    c.b + (t.b - c.b) * t.a,
                    1.0,
                ),
                None => c,
            }
        };

        for x in range.min_x..range.max_x {
            for y in range.min_y..range.max_y {
                let origin = Vec2::new(x as f32 * ts, y as f32 * ts);
                // Clamp the tile to the floor rect; a partial edge tile draws
                // its base tone only (details assume a full tile).
                let tw = (self.width - origin.x).min(ts);
                let th = (self.height - origin.y).min(ts);
                let full = tw >= ts && th >= ts;
                match self.surface {
                    Surface::Checker => {
                        // Carpet squares: two-tone purple checker with a woven
                        // border ring inside every tile (ring tone flips with
                        // the tile tone), a rare muted hot-pink mark and a
                        // rare soaked-in stain.
                        let a_tile = (x + y) % 2 == 0;
                        let raw = if a_tile {
                            palette::CHECKER_A
                        } else {
                            palette::CHECKER_B
                        };
                        let base = mix(raw);
                        graphics.draw_rectangle(origin, tw, th, base);
                        if !full {
                            continue;
                        }
                        let ring = mix(if a_tile {
                            palette::CHECKER_RING_DARK
                        } else {
                            palette::CHECKER_RING_LIGHT
                        });
                        graphics.draw_rectangle(
                            Vec2::new(origin.x + 3.0, origin.y + 3.0),
                            ts - 6.0,
                            ts - 6.0,
                            ring,
                        );
                        graphics.draw_rectangle(
                            Vec2::new(origin.x + 7.0, origin.y + 7.0),
                            ts - 14.0,
                            ts - 14.0,
                            base,
                        );
                        // Fibre variance: a few 1-art-pixel dots of the tile's
                        // own tone nudged ~4% either way, inside the ring —
                        // the pile catches the light differently dalle by
                        // dalle instead of reading dead flat.
                        for i in 0..3u32 {
                            let hx = tile_noise(x, y, 0x3100 + i);
                            let hy = tile_noise(x, y, 0x3200 + i);
                            let d = if tile_noise(x, y, 0x3300 + i) > 0.5 {
                                0.009
                            } else {
                                -0.009
                            };
                            graphics.draw_rectangle(
                                Vec2::new(
                                    origin.x + 7.0 + hx * (ts - 17.0),
                                    origin.y + 7.0 + hy * (ts - 17.0),
                                ),
                                3.0,
                                3.0,
                                mix(shade(raw, d)),
                            );
                        }
                        let n = tile_noise(x, y, 0xCA9);
                        if n > 0.93 {
                            // Hot carpet mark, centred on the tile.
                            graphics.draw_rectangle(
                                Vec2::new(origin.x + (ts - 7.0) * 0.5, origin.y + (ts - 7.0) * 0.5),
                                7.0,
                                7.0,
                                mix(palette::CHECKER_ACCENT),
                            );
                        } else if n < 0.06 {
                            let m = tile_noise(x, y, 0x77);
                            graphics.draw_rectangle(
                                Vec2::new(
                                    origin.x + 3.0 + 8.0 * m,
                                    origin.y + 3.0 + 7.0 * (1.0 - m),
                                ),
                                10.0,
                                7.0,
                                mix(palette::CHECKER_STAIN),
                            );
                        }
                    }
                    Surface::Asphalt => {
                        // Worn lot surface: three discrete base tones, fresh
                        // tar patches, faded lane-paint remnants and cracks.
                        let n = tile_noise(x, y, 0xA5F);
                        let base = if n < 0.34 {
                            palette::ASPHALT_DARK
                        } else if n < 0.67 {
                            palette::ASPHALT_MID
                        } else {
                            palette::ASPHALT_LIGHT
                        };
                        graphics.draw_rectangle(origin, tw, th, mix(base));
                        if !full {
                            continue;
                        }
                        // Aggregate speckle: 3-6 wu (1-2 art px) grains of the
                        // neighbour tones plus a rare bright chip — dense
                        // enough that the lot reads worn, not flat, both raw
                        // and at `?pixel=3`.
                        for i in 0..6u32 {
                            let hx = tile_noise(x, y, 0x4100 + i);
                            let hy = tile_noise(x, y, 0x4200 + i);
                            let hs = tile_noise(x, y, 0x4300 + i);
                            let ht = tile_noise(x, y, 0x4400 + i);
                            let s = if hs > 0.62 { 6.0 } else { 3.0 };
                            let tone = if ht < 0.06 {
                                palette::ASPHALT_AGGREGATE
                            } else if !(0.34..0.67).contains(&n) {
                                palette::ASPHALT_MID
                            } else if ht > 0.5 {
                                palette::ASPHALT_LIGHT
                            } else {
                                palette::ASPHALT_DARK
                            };
                            graphics.draw_rectangle(
                                Vec2::new(
                                    origin.x + 1.0 + hx * (ts - s - 2.0),
                                    origin.y + 1.0 + hy * (ts - s - 2.0),
                                ),
                                s,
                                s,
                                mix(tone),
                            );
                        }
                        let d = tile_noise(x, y, 0x77);
                        let m = tile_noise(x, y, 0x515);
                        if d > 0.84 {
                            graphics.draw_rectangle(
                                Vec2::new(origin.x + 2.0 + 5.0 * m, origin.y + 2.0 + 6.0 * m),
                                11.0 + 6.0 * m,
                                8.0 + 5.0 * (1.0 - m),
                                mix(palette::ASPHALT_PATCH),
                            );
                        } else if d < 0.06 {
                            let paint = if m > 0.5 {
                                palette::ASPHALT_PAINT
                            } else {
                                palette::ASPHALT_PAINT_WHITE
                            };
                            if tile_noise(x, y, 0x9E1) > 0.5 {
                                graphics.draw_rectangle(
                                    Vec2::new(origin.x + 4.0, origin.y + 5.0 + 13.0 * m),
                                    14.0,
                                    4.0,
                                    mix(paint),
                                );
                            } else {
                                graphics.draw_rectangle(
                                    Vec2::new(origin.x + 5.0 + 13.0 * m, origin.y + 4.0),
                                    4.0,
                                    14.0,
                                    mix(paint),
                                );
                            }
                        } else if d < 0.14 {
                            // Crack: a stepped L of two thin rects.
                            let cx = origin.x + ts * (0.2 + 0.5 * m);
                            let cy = origin.y + ts * 0.15;
                            graphics.draw_rectangle(
                                Vec2::new(cx, cy),
                                3.0,
                                ts * 0.45,
                                mix(palette::ASPHALT_CRACK),
                            );
                            graphics.draw_rectangle(
                                Vec2::new(cx - ts * 0.25, cy + ts * 0.45 - 3.0),
                                ts * 0.25,
                                3.0,
                                mix(palette::ASPHALT_CRACK),
                            );
                        }
                    }
                    Surface::Marble => {
                        // Pale two-tone slabs — pale for THIS palette (the
                        // checker floor sits at ~0.15), muted enough that the
                        // white HUD and the bots still read — with grout on
                        // the right / bottom edges, stepped vein accents and
                        // a rare brass inlay.
                        let n = tile_noise(x, y, 0x3AB);
                        let base = if (x + y) % 2 == 0 {
                            palette::MARBLE_A
                        } else {
                            palette::MARBLE_B
                        };
                        let d = (n - 0.5) * 0.025;
                        graphics.draw_rectangle(origin, tw, th, mix(shade(base, d)));
                        if !full {
                            continue;
                        }
                        let seam = mix(palette::MARBLE_SEAM);
                        graphics.draw_rectangle(
                            Vec2::new(origin.x + ts - 3.0, origin.y),
                            3.0,
                            ts,
                            seam,
                        );
                        graphics.draw_rectangle(
                            Vec2::new(origin.x, origin.y + ts - 3.0),
                            ts,
                            3.0,
                            seam,
                        );
                        // Rare 1-art-px polish glint — marble stays otherwise
                        // clean on purpose (polish = flat).
                        if tile_noise(x, y, 0x6E6) > 0.955 {
                            graphics.draw_rectangle(
                                Vec2::new(
                                    origin.x + 3.0 + tile_noise(x, y, 0x6F6) * (ts - 9.0),
                                    origin.y + 3.0 + tile_noise(x, y, 0x707) * (ts - 9.0),
                                ),
                                3.0,
                                3.0,
                                mix(palette::MARBLE_GLINT),
                            );
                        }
                        if n > 0.55 {
                            // A stepped diagonal vein: two offset thin rects.
                            let m = tile_noise(x, y, 0x51);
                            let vein = mix(if tile_noise(x, y, 0xBEE) > 0.5 {
                                palette::MARBLE_VEIN_LIGHT
                            } else {
                                palette::MARBLE_VEIN_DARK
                            });
                            let vx = origin.x + ts * (0.08 + 0.25 * m);
                            let vy = origin.y + ts * (0.15 + 0.4 * m);
                            graphics.draw_rectangle(Vec2::new(vx, vy), ts * 0.32, 3.0, vein);
                            graphics.draw_rectangle(
                                Vec2::new(vx + ts * 0.26, vy + 3.0),
                                ts * 0.28,
                                3.0,
                                vein,
                            );
                        } else if n < 0.035 {
                            graphics.draw_rectangle(
                                Vec2::new(origin.x + (ts - 5.0) * 0.5, origin.y + (ts - 5.0) * 0.5),
                                5.0,
                                5.0,
                                mix(palette::MARBLE_INLAY),
                            );
                        }
                    }
                    Surface::Concrete => {
                        // Poured slab: three discrete tones, expansion joints
                        // every 8 tiles (200 world units), broad stains,
                        // stepped cracks and a rare safety-paint remnant.
                        let n = tile_noise(x, y, 0xC0C);
                        let base = if n < 0.34 {
                            palette::CONCRETE_A
                        } else if n < 0.67 {
                            palette::CONCRETE_B
                        } else {
                            palette::CONCRETE_C
                        };
                        graphics.draw_rectangle(origin, tw, th, mix(base));
                        if !full {
                            continue;
                        }
                        let joint = mix(palette::CONCRETE_JOINT);
                        if x % 8 == 7 {
                            graphics.draw_rectangle(
                                Vec2::new(origin.x + ts - 3.0, origin.y),
                                3.0,
                                ts,
                                joint,
                            );
                        }
                        if y % 8 == 7 {
                            graphics.draw_rectangle(
                                Vec2::new(origin.x, origin.y + ts - 3.0),
                                ts,
                                3.0,
                                joint,
                            );
                        }
                        // Sparse cure speckle (neighbour-tone 1-art-px grains)
                        // + an occasional air-pocket pit — a poured slab is
                        // never perfectly flat.
                        for i in 0..3u32 {
                            let hx = tile_noise(x, y, 0x5100 + i);
                            let hy = tile_noise(x, y, 0x5200 + i);
                            let ht = tile_noise(x, y, 0x5300 + i);
                            let tone = if n >= 0.34 {
                                palette::CONCRETE_A
                            } else if ht > 0.5 {
                                palette::CONCRETE_B
                            } else {
                                palette::CONCRETE_C
                            };
                            graphics.draw_rectangle(
                                Vec2::new(
                                    origin.x + 1.0 + hx * (ts - 5.0),
                                    origin.y + 1.0 + hy * (ts - 5.0),
                                ),
                                3.0,
                                3.0,
                                mix(tone),
                            );
                        }
                        if tile_noise(x, y, 0x5A5) > 0.72 {
                            graphics.draw_rectangle(
                                Vec2::new(
                                    origin.x + 2.0 + tile_noise(x, y, 0x5B5) * (ts - 7.0),
                                    origin.y + 2.0 + tile_noise(x, y, 0x5C5) * (ts - 7.0),
                                ),
                                3.0,
                                3.0,
                                mix(palette::CONCRETE_PIT),
                            );
                        }
                        let d = tile_noise(x, y, 0xD17);
                        let m = tile_noise(x, y, 0x2F5);
                        if d > 0.82 {
                            graphics.draw_rectangle(
                                Vec2::new(origin.x + 2.0 + 7.0 * m, origin.y + 2.0 + 6.0 * m),
                                8.0 + 6.0 * m,
                                6.0 + 5.0 * (1.0 - m),
                                mix(palette::CONCRETE_STAIN),
                            );
                        } else if d < 0.09 {
                            let cx = origin.x + ts * (0.2 + 0.5 * m);
                            let cy = origin.y + ts * 0.12;
                            graphics.draw_rectangle(
                                Vec2::new(cx, cy),
                                3.0,
                                ts * 0.5,
                                mix(palette::CONCRETE_CRACK),
                            );
                            graphics.draw_rectangle(
                                Vec2::new(cx, cy + ts * 0.5 - 3.0),
                                ts * 0.3,
                                3.0,
                                mix(palette::CONCRETE_CRACK),
                            );
                        } else if d < 0.115 {
                            graphics.draw_rectangle(
                                Vec2::new(origin.x + 4.0 + 10.0 * m, origin.y + 3.0 + 12.0 * m),
                                7.0,
                                3.0,
                                mix(palette::CONCRETE_MARK),
                            );
                        }
                    }
                    Surface::Grating => {
                        // Dark deck with a chunky mesh (3 bars per tile, ~8 px
                        // cells): the void shows through, the top/left bars of
                        // every tile catch the light (module edges) and
                        // hash-picked tiles are solid riveted plates.
                        let n = tile_noise(x, y, 0x6A7);
                        if n > 0.88 {
                            graphics.draw_rectangle(origin, tw, th, mix(palette::GRATING_PLATE));
                            if !full {
                                continue;
                            }
                            let rivet = mix(palette::GRATING_RIVET);
                            for (rx, ry) in [
                                (3.0, 3.0),
                                (ts - 6.0, 3.0),
                                (3.0, ts - 6.0),
                                (ts - 6.0, ts - 6.0),
                            ] {
                                graphics.draw_rectangle(
                                    Vec2::new(origin.x + rx, origin.y + ry),
                                    3.0,
                                    3.0,
                                    rivet,
                                );
                            }
                        } else {
                            graphics.draw_rectangle(origin, tw, th, mix(palette::GRATING_VOID));
                            if !full {
                                continue;
                            }
                            let bar = mix(palette::GRATING_BAR);
                            let lit = mix(palette::GRATING_BAR_LIT);
                            let step = ts / 3.0;
                            for i in 0..3 {
                                let o = i as f32 * step;
                                let tone = if i == 0 { lit } else { bar };
                                graphics.draw_rectangle(
                                    Vec2::new(origin.x + o, origin.y),
                                    3.0,
                                    ts,
                                    tone,
                                );
                                graphics.draw_rectangle(
                                    Vec2::new(origin.x, origin.y + o),
                                    ts,
                                    3.0,
                                    tone,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
