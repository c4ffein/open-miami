//! `render_hud`: the SCREEN-SPACE layer of an in-game frame, drawn after
//! `render::world::render_world` — the HUD (or the extraction card), the
//! scenario's hold caption / gate prompt / dialogue panel, the hold-R
//! restart bar, the pixel crosshair, the TV-static grain.
//!
//! A pure function of a [`HudView`] (read-only state) -> draw commands: no
//! input, no mutation, no browser — see docs/ARCHITECTURE.md. The app builds
//! the view each frame (`GameState::update_game`), sampling the mouse itself;
//! `tests/render_stream.rs` builds views natively and validates the stream.

use super::comms::{render_gate_prompt, render_hold_caption};
use super::dialogue::render_dialogue;
use super::render_ui;
use crate::components::WeaponType;
use crate::graphics::Graphics;
use crate::hud_msg::MsgRoller;
use crate::math::{Color, Vec2};
use crate::scenario::ScenarioState;

/// The "EXFILTRATED // FLOOR N" card that replaces the HUD once the player
/// has extracted (`ending::draw_extract_card`).
pub struct ExtractCard<'a> {
    pub floor_title: &'a str,
    /// Seconds since the extraction.
    pub t: f32,
    /// 1 = opaque; the last floor's outro fades it out.
    pub alpha: f32,
    /// The exit leads to the surface (the ending) rather than the next floor.
    pub home: bool,
}

/// Everything the screen-space layer reads. Built by the caller every frame.
pub struct HudView<'a> {
    /// `Some` = the floor is complete: the card draws INSTEAD of the HUD, and
    /// the scenario overlays are suppressed.
    pub extract_card: Option<ExtractCard<'a>>,
    pub ammo: i32,
    pub weapon: Option<WeaponType>,
    /// The ammo box's eased slide, 0 (in place) .. 1 (off-screen).
    pub ammo_slide: f32,
    pub enemies_alive: usize,
    pub player_alive: bool,
    /// Seconds since the player died (drives the death screen).
    pub death_time: f32,
    pub debug_enabled: bool,
    pub show_infos: bool,
    pub roller: &'a MsgRoller,
    /// The running scenario: its hold caption, gate prompt and dialogue.
    pub scenario: Option<&'a ScenarioState>,
    /// Hold-R restart progress 0..1 (`None` = bar hidden).
    pub restart_progress: Option<f32>,
    /// Where the pixel crosshair sits — the mouse, in CSS px (the APP
    /// samples the input; drawing never does).
    pub cursor: Vec2,
    /// Opacity of the in-game TV-static grain (POSTFX 13); `None` = off
    /// (`?noise=0`). Emitted LAST so a later POSTFX (the outro's blur-out)
    /// can still replace it: only the last POSTFX of a frame applies.
    pub tv_static: Option<f32>,
    /// The floor's accent colour.
    pub accent: (u8, u8, u8),
    /// The continuous animation clock, seconds.
    pub now: f32,
}

pub fn render_hud(graphics: &Graphics, v: &HudView) {
    // The UI — or, once extracted, the "EXFILTRATED // FLOOR N" card (which
    // the outro fades out on the last floor).
    let level_complete = v.extract_card.is_some();
    if let Some(card) = &v.extract_card {
        crate::ending::draw_extract_card(graphics, card.floor_title, card.t, card.alpha, card.home);
    } else {
        render_ui(
            graphics,
            v.ammo,
            v.weapon,
            v.ammo_slide,
            v.enemies_alive,
            v.player_alive,
            v.death_time,
            v.debug_enabled,
            v.show_infos,
            v.roller,
            v.now,
        );
    }

    // The scenario's screen-space overlays. (The old top-left "> OBJECTIVE"
    // prose block and the bottom-left intercepted-comms ticker are retired:
    // the top-right message roller carries the directive and the dialogue
    // panel is the one place conversations render. `say` lines still queue /
    // type invisibly so `hold.until_comms_idle` timing and the epilogue's
    // feed-idle detection keep working.)
    if let Some(sc) = v.scenario {
        let live = v.player_alive && !level_complete;
        // The caption of a running `hold`, if it has one.
        if let Some(text) = sc.hold_caption() {
            if live {
                render_hold_caption(graphics, text, v.accent, sc.time());
            }
        }
        // The tutorial gate prompt ("LEFT CLICK — PUNCH"): a centred
        // lower-third caption while the world is frozen on a gate.
        if let Some(g) = sc.gate_view() {
            if live {
                render_gate_prompt(graphics, &g, v.accent, v.now);
            }
        }
        // The visual-novel dialogue panel (`talk` conversations), over
        // everything else on the HUD layer.
        if let Some(view) = sc.dialogue_view() {
            if live {
                render_dialogue(graphics, &view, v.accent, v.now);
            }
        }
    }

    // The hold-R restart load bar, centre screen: outline + accent fill by
    // progress.
    if let Some(t) = v.restart_progress {
        let (w, h) = (graphics.width(), graphics.height());
        let (bw, bh) = (220.0, 10.0);
        let (bx, by) = ((w - bw) / 2.0, (h - bh) / 2.0 - 40.0);
        graphics.draw_rectangle(
            Vec2::new(bx - 2.0, by - 2.0),
            bw + 4.0,
            bh + 4.0,
            Color::new(0.0, 0.0, 0.0, 0.55),
        );
        graphics.draw_rectangle_lines(
            Vec2::new(bx, by),
            bw,
            bh,
            1.0,
            Color::new(0.9, 0.9, 0.9, 0.8),
        );
        graphics.draw_rectangle(
            Vec2::new(bx + 2.0, by + 2.0),
            (bw - 4.0) * t,
            bh - 4.0,
            Color::new(
                v.accent.0 as f32 / 255.0,
                v.accent.1 as f32 / 255.0,
                v.accent.2 as f32 / 255.0,
                0.95,
            ),
        );
        graphics.draw_text(
            "RESTARTING",
            Vec2::new(bx + bw / 2.0 - 92.0, by - 44.0),
            36.0,
            Color::new(0.95, 0.95, 0.95, 0.9),
        );
    }

    // The pixel crosshair replacing the OS cursor: a 7x7 cross with an empty
    // centre cell, drawn last so it sits over everything.
    {
        let m = v.cursor;
        let cell = 3.0;
        let origin = Vec2::new((m.x - 3.5 * cell).floor(), (m.y - 3.5 * cell).floor());
        let c = Color::new(1.0, 1.0, 1.0, 0.92);
        for i in 0..7 {
            if i == 3 {
                continue; // empty centre pixel
            }
            graphics.draw_rectangle(
                Vec2::new(origin.x + 3.0 * cell, origin.y + i as f32 * cell),
                cell,
                cell,
                c,
            );
            graphics.draw_rectangle(
                Vec2::new(origin.x + i as f32 * cell, origin.y + 3.0 * cell),
                cell,
                cell,
                c,
            );
        }
    }

    // The title screen's faint TV-static shimmer, over every in-game frame
    // (world + HUD alike — POSTFX is frame-level).
    if let Some(t) = v.tv_static {
        graphics.postfx(13, t, Color::WHITE);
    }
}
