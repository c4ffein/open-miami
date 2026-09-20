//! The neon OPEN / MIAMI title: the 12x16 glyphs (single source of truth —
//! `tools/gen_title.py` parses `title_glyph` out of THIS file for the loading
//! screen's inline SVG) and their in-game renderer.

use crate::graphics::Graphics;
use crate::math::{Color, Vec2};

/// One 12x16 pixel bitmap per title glyph ('#' = filled). The neon look
/// comes from drawing only the BOUNDARY cells of these fat letterforms:
/// that yields the outer contour and, where a glyph has a counter (O, P,
/// A), the inner contour — two neon lines with an empty letter between.
pub fn title_glyph(ch: char) -> [&'static str; 16] {
    match ch {
        'O' => [
            ".##########.",
            "############",
            "############",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "############",
            "############",
            ".##########.",
        ],
        'P' => [
            "###########.",
            "############",
            "############",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "############",
            "############",
            "###########.",
            "###.........",
            "###.........",
            "###.........",
            "###.........",
            "###.........",
            "###.........",
        ],
        'E' => [
            "###########.",
            "############",
            "############",
            "###.........",
            "###.........",
            "###.........",
            "##########..",
            "##########..",
            "##########..",
            "###.........",
            "###.........",
            "###.........",
            "###.........",
            "############",
            "############",
            "###########.",
        ],
        'N' => [
            "#####....###",
            "#####....###",
            "#####....###",
            "###.##...###",
            "###.##...###",
            "###..##..###",
            "###..##..###",
            "###..##..###",
            "###...##.###",
            "###...##.###",
            "###....#####",
            "###....#####",
            "###....#####",
            "###.....####",
            "###.....####",
            "###.....####",
        ],
        'M' => [
            "#####..#####",
            "#####..#####",
            "###.####.###",
            "###.####.###",
            "###..##..###",
            "###..##..###",
            "###..##..###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
        ],
        'I' => [
            ".##########.",
            ".##########.",
            ".##########.",
            "....####....",
            "....####....",
            "....####....",
            "....####....",
            "....####....",
            "....####....",
            "....####....",
            "....####....",
            "....####....",
            "....####....",
            ".##########.",
            ".##########.",
            ".##########.",
        ],
        'A' => [
            ".##########.",
            "############",
            "############",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "############",
            "############",
            "############",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
        ],
        // The loading screen's extra glyphs (tools/gen_title.py renders
        // "LOADING..." out of this same table).
        'L' => [
            "###.........",
            "###.........",
            "###.........",
            "###.........",
            "###.........",
            "###.........",
            "###.........",
            "###.........",
            "###.........",
            "###.........",
            "###.........",
            "###.........",
            "###.........",
            "############",
            "############",
            "###########.",
        ],
        'D' => [
            "##########..",
            "###########.",
            "############",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "############",
            "###########.",
            "##########..",
        ],
        'G' => [
            ".##########.",
            "############",
            "############",
            "###.........",
            "###.........",
            "###.........",
            "###.........",
            "###....#####",
            "###....#####",
            "###......###",
            "###......###",
            "###......###",
            "###......###",
            "############",
            "############",
            ".##########.",
        ],
        '.' => [
            "............",
            "............",
            "............",
            "............",
            "............",
            "............",
            "............",
            "............",
            "............",
            "............",
            "............",
            "..####......",
            "..####......",
            "..####......",
            "..####......",
            "............",
        ],
        _ => ["............"; 16],
    }
}

/// The title: "OPEN" / "MIAMI" as huge hollow neon-pink pixel letters
/// (outer + inner contours of the fat glyphs, with a two-ring pixel glow
/// around them), rasterized in one pixel-art group opened UNDER a slow
/// rotation — the whole sign sways between -12 and -3 degrees.
pub fn draw_neon_title(graphics: &Graphics, cx: f32, cy: f32, t: f32) {
    const UNIT: f32 = 8.0; // one art pixel = 8 screen px
    const GW: usize = 72;
    const GH: usize = 36;
    let mut filled = [[false; GW]; GH];
    let stamp = |word: &str, x0: usize, y0: usize, filled: &mut [[bool; GW]; GH]| {
        for (i, ch) in word.chars().enumerate() {
            let glyph = title_glyph(ch);
            let gx = x0 + i * 14; // 12 wide + 2 gap
            for (r, row) in glyph.iter().enumerate() {
                for (c, cell) in row.bytes().enumerate() {
                    if cell == b'#' {
                        filled[y0 + r][gx + c] = true;
                    }
                }
            }
        }
    };
    stamp("OPEN", 8, 1, &mut filled);
    stamp("MIAMI", 1, 19, &mut filled);

    let at = |r: isize, c: isize| -> bool {
        r >= 0 && c >= 0 && (r as usize) < GH && (c as usize) < GW && filled[r as usize][c as usize]
    };
    // Boundary = a filled cell with an empty 4-neighbour; the glow rings
    // are the empty cells within 1 / 2 (8-neighbourhood) of a boundary.
    let mut layer = [[0u8; GW]; GH]; // 3 = core, 2 = glow, 1 = faint glow
    for r in 0..GH as isize {
        for c in 0..GW as isize {
            if at(r, c) && !(at(r - 1, c) && at(r + 1, c) && at(r, c - 1) && at(r, c + 1)) {
                layer[r as usize][c as usize] = 3;
            }
        }
    }
    for pass in [2u8, 1u8] {
        let want = pass + 1;
        for r in 0..GH as isize {
            for c in 0..GW as isize {
                if at(r, c) || layer[r as usize][c as usize] != 0 {
                    continue;
                }
                'scan: for dr in -1..=1 {
                    for dc in -1..=1 {
                        let (nr, nc) = (r + dr, c + dc);
                        if nr >= 0
                            && nc >= 0
                            && (nr as usize) < GH
                            && (nc as usize) < GW
                            && layer[nr as usize][nc as usize] == want
                        {
                            layer[r as usize][c as usize] = pass;
                            break 'scan;
                        }
                    }
                }
            }
        }
    }

    // Slow sway between -12 and -3 degrees (period ~20 s).
    let ang = (-7.5 + 4.5 * (t * 0.31).sin()).to_radians();
    let (w, h) = (GW as f32 * UNIT, GH as f32 * UNIT);
    graphics.save();
    graphics.translate(cx, cy);
    graphics.rotate(ang);
    graphics.pixel_begin(UNIT, w, h);
    for (r, row) in layer.iter().enumerate() {
        for (c, &l) in row.iter().enumerate() {
            if l == 0 {
                continue;
            }
            let color = match l {
                3 => Color::new(1.0, 0.20, 0.60, 1.0),
                2 => Color::new(1.0, 0.20, 0.60, 0.30),
                _ => Color::new(1.0, 0.20, 0.60, 0.12),
            };
            graphics.draw_rectangle(
                Vec2::new(c as f32 * UNIT, r as f32 * UNIT),
                UNIT,
                UNIT,
                color,
            );
        }
    }
    graphics.pixel_end(-w / 2.0, -h / 2.0);
    graphics.restore();
}
