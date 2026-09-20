//! The browser app: `GameState` (every screen of the game + the `?viz`
//! toolbox), the `requestAnimationFrame` loop and the wasm entry point.
//!
//! Split by screen: `game_loop` (the in-game frame), `world_render`
//! (`render_world`), `robots` (the actors layer), `menus`, `title`, `viz/*`,
//! `url`, `perf`. Submodules `use super::*` and add `impl GameState` blocks;
//! `GameState`'s fields are private to this module tree.

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// Import game modules
use crate::audio::{ending_song, song_for_floor, AudioEngine};
use crate::camera::Camera;
use crate::ecs::{System, World};
use crate::ending::{self, Ending, Outro, EXTRACT_CARD_SECS};
use crate::game::*;
use crate::graphics::Graphics;
use crate::input;
use crate::level::Level;
use crate::levels::{floor_def, floor_title, level_index_for_floor_id, BOSS_LEVEL, LEVEL_COUNT};
use crate::math::{Color, Vec2};
use crate::props::{
    draw_prop_ex, family_range, largest_family, prop_family, prop_layers, prop_modes, prop_px,
    settings_json, snap_size, PixelMode, PropDrawOpts, MAX_LAYERS, MAX_PX, PROP_COUNT,
    PROP_FAMILIES, PROP_NAMES,
};
use crate::render::comms::{
    render_elevators, render_gate_prompt, render_hold_caption, render_zones_debug,
};
use crate::render::dialogue::render_dialogue;
use crate::render::*;
use crate::scenario::{ScenarioState, SURFACE_EXIT};
use crate::systems::boss::any_boss_enraged;
use crate::systems::*;

mod game_loop;
mod menus;
mod perf;
mod robots;
mod title;
mod url;
mod viz;
mod world_render;

use robots::*;
use title::*;
use url::*;
use viz::*;
use world_render::*;

/// Longest simulation step a single frame may take (seconds).
const MAX_FRAME_DT: f32 = 0.1;
/// Hold R this long (seconds) while alive to restart the floor.
const RESTART_HOLD_SECS: f32 = 1.0;
/// Safety cap for the loading screen's PRECOMPUTING step: if the audio
/// pre-renders have not finished by then (broken OfflineAudioContext,
/// pathologically slow machine), the game starts anyway — every sound
/// falls back to live synthesis until its bake lands.
const PRECOMPUTE_CAP_MS: f64 = 6000.0;
/// The faint TV-static shimmer (POSTFX kind 13) over the title screen —
/// the modals' static at a twelfth of its coverage.
const TV_STATIC_T: f32 = 0.9 / 12.0;
/// The same shimmer over in-game frames, dimmer than the title's
/// (0.5/12 vs 0.9/12) so it never fights the action for attention.
const TV_STATIC_GAME_T: f32 = 0.5 / 12.0;

#[wasm_bindgen]
extern "C" {
    // ?viz inspector panel: open the right-hand iframe on a gallery item /
    // hide it again (both defined in index.html).
    #[wasm_bindgen(js_namespace = window, js_name = vizInspect)]
    fn viz_inspect(kind: &str);
    #[wasm_bindgen(js_namespace = window, js_name = vizInspectHide)]
    fn viz_inspect_hide();
    // ?viz PROPS page SAVE: PUT the props/props.json document through
    // serve.py's editor API (token flow + result toast in index.html).
    #[wasm_bindgen(js_namespace = window, js_name = vizSaveProps)]
    fn viz_save_props(json: &str);
    // Open an external link in a new tab (defined in index.html).
    #[wasm_bindgen(js_namespace = window, js_name = openExternal)]
    fn open_external(url: &str);
    // The HTML loading overlay: progress during the PRECOMPUTING
    // (audio pre-render) step, and the hide call once the game may show
    // its first screen (both defined in index.html).
    #[wasm_bindgen(js_namespace = window, js_name = loadingProgress)]
    fn loading_progress(done: u32, total: u32);
    #[wasm_bindgen(js_namespace = window, js_name = loadingDone)]
    fn loading_done();
    // Persistent settings (localStorage; defined in index.html).
    #[wasm_bindgen(js_namespace = window, js_name = getSetting)]
    fn get_setting(name: &str) -> Option<String>;
    #[wasm_bindgen(js_namespace = window, js_name = setSetting)]
    fn set_setting(name: &str, value: &str);
    // Hide / restore the OS cursor over the canvas (defined in
    // index.html); hidden during gameplay, where the engine draws its
    // own pixel crosshair instead.
    #[wasm_bindgen(js_namespace = window, js_name = setCursorHidden)]
    fn set_cursor_hidden(hidden: bool);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameScreen {
    LevelSelect,
    BossIntro,
    InGame,
    Paused,
    Settings,
    About,
    Visualizer,
    /// The credits roll after the last car goes up (see `ending.rs`).
    Ending,
}

/// A mid-floor `checkpoint` snapshot: the full world (entities,
/// components, walls, RNG) plus the scenario state (fired steps, opened
/// exits, comms, objective) at the moment the action ran. Restored on
/// death instead of a full floor restart.
struct Checkpoint {
    world: World,
    scenario: ScenarioState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuOption {
    Play,
    Settings,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PauseOption {
    Continue,
    Settings,
    Stop,
}

struct GameState {
    screen: GameScreen,
    selected_level: usize,
    selected_menu_option: MenuOption,
    selected_pause_option: PauseOption,
    world: World,
    /// The gameplay systems in their one canonical order (shared with
    /// the headless sim — see `sim::GameSystems`).
    systems: crate::sim::GameSystems,
    elevator_system: ElevatorSystem,
    /// The running floor scenario (steps, comms feed, objective).
    scenario: Option<ScenarioState>,
    /// The latest mid-floor `checkpoint` snapshot: death restores it
    /// instead of restarting the floor. Cleared on every floor load.
    checkpoint: Option<Checkpoint>,
    /// Set once the player has extracted: the destination floor id
    /// (`SURFACE_EXIT` = surface). The completion card plays, then the
    /// floor loads.
    extracting: Option<usize>,
    /// Seconds R has been held while alive: at [`RESTART_HOLD_SECS`] the
    /// floor restarts from scratch (a load bar fills at screen centre).
    restart_hold: f32,
    /// The R that respawned the player (death → checkpoint) is still
    /// held: the hold-to-restart stays disarmed until it is released.
    restart_needs_release: bool,
    /// Whether the OS cursor is currently hidden over the canvas (the
    /// in-game pixel crosshair replaces it; menus keep the OS cursor).
    cursor_hidden: bool,
    /// Whether the music is stopped because a tutorial gate froze the
    /// world (restarted when the gate releases).
    music_frozen: bool,
    /// The pause menu's stacked SETTINGS modal is open (Esc pops one
    /// layer: settings -> pause -> game).
    pause_in_settings: bool,
    /// The PRECOMPUTING step: the HTML loading overlay stays up while
    /// the audio pre-render queue burns down at full budget, BEFORE the
    /// first screen shows (works for any entry, `?floor=N` included).
    /// Ends on completion or at the [`PRECOMPUTE_CAP_MS`] safety cap.
    precomputing: bool,
    precompute_started: f64,
    /// FPS counter (`?debug` only): frames counted since `fps_window`
    /// started, the window's start time (ms), and the last readout.
    fps_frames: u32,
    fps_window: f64,
    fps_value: f32,
    /// The SETTINGS frame-rate cap (30 / 60 / 120; 0 = uncapped).
    /// rAF can never EXCEED the display refresh — the cap only skips
    /// frames when the display is faster than the cap.
    fps_cap: u32,
    /// When the last non-skipped frame ran (ms), for the cap.
    last_frame_ms: f64,
    /// Which SETTINGS row is highlighted (0 = SOUND, 1 = FPS CAP).
    settings_row: usize,
    level: Level,
    camera: Camera,
    last_time: f64,
    /// Last time (ms) the canvas backing size was checked against the
    /// window (see `Graphics::sync_size`); polled about once a second.
    last_size_check: f64,
    death_time: f32,
    level_complete_time: f32,
    /// Debug tooling (I overlays, K purge, B crack): only with `?debug`.
    debug_enabled: bool,
    show_infos: bool,
    // Audio + the previous-frame state used to fire one-shot sound effects.
    audio: AudioEngine,
    /// The AudioContext has been resumed after a user gesture.
    audio_unlocked: bool,
    /// The post-extraction epilogue on the last floor (uplink comms, then
    /// the blur-out), `None` otherwise.
    outro: Option<Outro>,
    /// The credits screen clock.
    ending: Ending,
    boss_intro_line: usize,
    viz_tab: VizTab,
    /// Index of the sprites-gallery item open in the inspector (-1 = none).
    viz_selected: i32,
    /// SPRITES tab sub-page: false = characters, true = the prop library.
    viz_props_page: bool,
    /// MUSICS tab sub-page: false = the tracker, true = the SFX board.
    viz_sounds_page: bool,
    /// Selected prop in the PROPS gallery (big live preview on the right).
    viz_prop_selected: usize,
    /// PROPS gallery page: the prop FAMILY shown in the tile grid (an
    /// index into `PROP_FAMILIES`: DATACENTER / OUTDOOR / LOBBY).
    viz_prop_family: usize,
    /// PROPS gallery: per-prop pixel size / layer visibility / layer
    /// pixel modes (one entry per prop, see [`PropViz`]).
    viz_props: Vec<PropViz>,
    /// PROPS gallery "GRID": overlay the art-pixel grid on the preview.
    viz_pixel_grid: bool,
    /// `?pixel=N`: rasterize the in-game SCENERY (floor, walls, props,
    /// elevators — actors and HUD stay native-smooth) in a world-anchored
    /// pixel group of N-world-unit art pixels. 0 = off (the default).
    pixel_world: u32,
    /// `?noise=0` turns the TV-static film grain off (title screen and
    /// in-game alike) — an A/B switch for judging the pixelated world
    /// with a clean image. Default true (docs/URL_PARAMS.md).
    noise_enabled: bool,
    /// LEVELS tab: the native level editor (`editor_ui.rs`).
    editor: crate::editor_ui::Editor,
    /// EFFECTS tab: the running preview — -1 = the 2D shoggoth glitch,
    /// >= 0 = an index into [`POSTFX_PREVIEWS`]. Timed from `effect_start`.
    effect_kind: i32,
    effect_start: f64,
    prev_player_alive: bool,
    /// Seconds left on the kill flash (background strobes red/blue).
    kill_flash: f32,
    /// Electric spark bursts popping where player attacks land on bots
    /// (fixed-capacity ring, spawned from `EnemyHit` events, drawn in the
    /// actors layer of `render_world`).
    sparks: crate::sparks::SparkPool,
    /// The sliding bottom-left ammo box: slide offset / target / armed
    /// flag (`hud_ammo::AmmoSlide`; the box itself is drawn by
    /// `render::render_ammo_box` on the HUD layer).
    ammo_hud: crate::hud_ammo::AmmoSlide,
    /// The top-right MESSAGE ROLLER: the short chromatic directive box
    /// (`hud_msg::MsgRoller`; drawn by `render::render_msg_roller`).
    /// Rolls down for a few seconds when the objective changes or an
    /// exit opens, then rolls away.
    msg_roller: crate::hud_msg::MsgRoller,
    /// Key of the static geometry cache (floor tiles + walls baked into a
    /// persistent renderer-side VBO, `Graphics::static_layer`). Bumped by
    /// every `load_floor` so a floor change re-records; a checkpoint
    /// restore keeps it (same floor — the tiles and walls are identical).
    floor_static_key: u32,
    prev_boss_enraged: bool,
    prev_all_dead: bool,
}

impl GameState {
    fn new() -> Self {
        let screen = if wants_visualizer() {
            GameScreen::Visualizer
        } else {
            GameScreen::LevelSelect
        };
        let mut state = GameState {
            screen,
            selected_level: 0,
            selected_menu_option: MenuOption::Play,
            selected_pause_option: PauseOption::Continue,
            world: World::new(),
            systems: crate::sim::GameSystems::default(),
            elevator_system: ElevatorSystem,
            scenario: None,
            checkpoint: None,
            extracting: None,
            restart_hold: 0.0,
            restart_needs_release: false,
            cursor_hidden: false,
            music_frozen: false,
            pause_in_settings: false,
            precomputing: true,
            precompute_started: 0.0,
            fps_frames: 0,
            fps_window: 0.0,
            fps_value: 0.0,
            fps_cap: get_setting("fps_cap")
                .and_then(|v| v.parse().ok())
                .unwrap_or(120),
            last_frame_ms: 0.0,
            settings_row: 0,
            level: Level::new(),
            camera: Camera::new(),
            last_time: 0.0,
            last_size_check: 0.0,
            death_time: 0.0,
            level_complete_time: 0.0,
            debug_enabled: url_flag("debug"),
            show_infos: false,
            audio: {
                let audio = AudioEngine::new();
                // The SETTINGS sound toggle persists in localStorage.
                if get_setting("sound").as_deref() == Some("off") {
                    audio.set_enabled(false);
                }
                audio
            },
            audio_unlocked: false,
            outro: None,
            ending: Ending::new(),
            boss_intro_line: 0,
            viz_tab: VizTab::Sprites,
            viz_selected: -1,
            viz_props_page: false,
            viz_sounds_page: false,
            viz_prop_selected: 0,
            viz_prop_family: 0,
            viz_props: (0..PROP_COUNT)
                .map(|k| PropViz {
                    px: prop_px(k),
                    visible: u32::MAX,
                    modes: prop_modes(k),
                })
                .collect(),
            viz_pixel_grid: false,
            pixel_world: url_param("pixel")
                .and_then(|v| v.parse::<u32>().ok())
                .filter(|&n| n >= 2)
                .unwrap_or(0),
            noise_enabled: !matches!(
                url_param("noise").as_deref(),
                Some("0") | Some("false") | Some("off")
            ),
            editor: crate::editor_ui::Editor::new(),
            effect_kind: -1,
            effect_start: 0.0,
            prev_player_alive: true,
            kill_flash: 0.0,
            sparks: crate::sparks::SparkPool::new(),
            ammo_hud: crate::hud_ammo::AmmoSlide::new(),
            msg_roller: crate::hud_msg::MsgRoller::new(),
            floor_static_key: 0,
            prev_boss_enraged: false,
            prev_all_dead: false,
        };
        // `?floor=N`: jump straight into that floor (editor "play" button,
        // testing). Audio stays off until the first user gesture.
        if !wants_visualizer() {
            if url_flag("ending") {
                // `?ending`: jump straight to the credits (dev shortcut,
                // same spirit as `?floor=N`; the ride home, `ending::render_ride`).
                state.screen = GameScreen::Ending;
            } else if let Some(level) = Self::url_start_floor() {
                state.selected_level = level;
                state.start_game();
            }
        }
        state
    }

    /// `?floor=N` in the URL (the floor id: 0 = the ground-level cold
    /// open, 1..13, 14 = 13½): the level index to start on directly, if
    /// present and valid.
    fn url_start_floor() -> Option<usize> {
        let n: usize = url_param("floor")?.parse().ok()?;
        level_index_for_floor_id(n)
    }

    /// (Re)build the world for `selected_level` and start its scenario.
    fn load_floor(&mut self) {
        // New floor geometry: invalidate the static tiles+walls cache
        // (the renderer evicts the old VBO when the new key records).
        // Death restarts re-load the same floor — the content would be
        // identical, but bumping is always correct and costs one record.
        self.floor_static_key = self.floor_static_key.wrapping_add(1);
        self.world.clear();
        initialize_game(&mut self.world, self.selected_level);
        self.scenario = Some(ScenarioState::new(floor_def(self.selected_level)));
        self.checkpoint = None;
        let def = floor_def(self.selected_level);
        self.level.set_surface(def.surface);
        // Clip the tile field to the floor's playable rect: outside it
        // the neon-wave void backdrop shows (render_world draws it
        // first, the tiles cover it inside the level).
        self.level.set_size(def.width, def.height);
        self.reset_run_state();
    }

    /// Restore the latest mid-floor `checkpoint` (same floor): the world
    /// and scenario come back exactly as snapshotted. Returns whether a
    /// checkpoint existed.
    fn restore_checkpoint(&mut self) -> bool {
        let Some(cp) = &self.checkpoint else {
            return false;
        };
        self.world = cp.world.clone();
        self.scenario = Some(cp.scenario.clone());
        self.reset_run_state();
        true
    }

    /// Shared tail of `load_floor` / `restore_checkpoint`: camera, run
    /// flags, and the previous-frame sound-effect trackers (seeded from
    /// the fresh world so the first frame fires no spurious sounds).
    fn reset_run_state(&mut self) {
        self.camera.set_cinematic(None);
        self.extracting = None;
        self.outro = None;
        self.death_time = 0.0;
        self.level_complete_time = 0.0;
        self.kill_flash = 0.0;
        self.sparks.clear();
        // The ammo box snaps to the fresh world's held weapon: no slide
        // animation on a floor load / checkpoint restore.
        self.ammo_hud
            .snap(crate::hud_ammo::gun_held(get_player_weapon(&self.world)));
        // Fresh roller: the floor's (restored) objective re-announces
        // itself on the first update — a load / restore restating the
        // current directive is the wanted behaviour.
        self.msg_roller = crate::hud_msg::MsgRoller::new();
        self.prev_player_alive = is_player_alive(&self.world);
        self.prev_boss_enraged = any_boss_enraged(&self.world);
        self.prev_all_dead = count_alive_enemies(&self.world) == 0;
    }

    fn start_game(&mut self) {
        self.load_floor();

        // Music (re)starts with every floor, on that floor's song. The
        // Enter keypress that got us here is a user gesture, so audio may
        // start; a `?floor=N` session has had none yet — `update` resumes
        // the context on the first in-game key/click instead.
        self.audio.resume();
        // Songs escalate by depth, keyed on the floor ID (0 = the gate
        // cold open, 13 / 14 = the boss floors).
        self.audio
            .set_song(song_for_floor(floor_def(self.selected_level).id));
        self.audio.start_music();

        // The hidden floor opens with a face-off before the fight.
        if self.selected_level == BOSS_LEVEL {
            self.boss_intro_line = 0;
            self.screen = GameScreen::BossIntro;
        } else {
            self.screen = GameScreen::InGame;
        }
    }

    fn update(&mut self, graphics: &Graphics, current_time: f64) {
        let dt = if self.last_time == 0.0 {
            0.016 // Initial frame assume 60fps
        } else {
            // Clamp long frames (first-frame atlas baking, tab switches,
            // headless renderers) so actors cannot tunnel through walls
            // or teleport across the floor in a single step.
            (((current_time - self.last_time) / 1000.0) as f32).min(MAX_FRAME_DT)
        };
        self.last_time = current_time;

        // PRECOMPUTING: while the HTML loading overlay is still up, burn
        // the audio pre-render queue down at full budget and report
        // progress — the first screen only shows once every voice is
        // baked (or the safety cap fires). No screen runs, no input is
        // consumed; works identically for `?floor=N` starts.
        if self.precomputing {
            if self.precompute_started == 0.0 {
                self.precompute_started = current_time;
            }
            self.audio.set_pump_budget(8);
            self.audio.update(current_time / 1000.0);
            let (done, total) = self.audio.bake_progress();
            loading_progress(done, total);
            if self.audio.bake_complete()
                || current_time - self.precompute_started >= PRECOMPUTE_CAP_MS
            {
                self.precomputing = false;
                loading_done();
            }
            input::end_frame();
            return;
        }

        // Follow window resizes, browser zoom and DPR changes (reading
        // layout sizes can force style work, so poll ~1/s, not per frame).
        if current_time - self.last_size_check >= 1000.0 {
            self.last_size_check = current_time;
            graphics.sync_size();
        }

        // Clear background
        graphics.clear(Color::new(20.0 / 255.0, 12.0 / 255.0, 28.0 / 255.0, 1.0));

        // Browsers keep the AudioContext suspended until a user gesture:
        // unlock it on the first key/click anywhere (matters for `?floor=N`
        // sessions, which start in-game without the menu's Enter).
        if !self.audio_unlocked && input::any_pressed() {
            self.audio.resume();
            self.audio_unlocked = true;
        }

        // In-game the OS cursor is hidden and the engine draws its own
        // pixel crosshair (update_game); every other screen (menus, the
        // ?viz toolbox, the editor) keeps the native pointer.
        let want_hidden = self.screen == GameScreen::InGame;
        if want_hidden != self.cursor_hidden {
            set_cursor_hidden(want_hidden);
            self.cursor_hidden = want_hidden;
        }

        match self.screen {
            GameScreen::LevelSelect => {
                self.update_level_select(graphics);
            }
            GameScreen::BossIntro => {
                self.update_boss_intro(graphics);
            }
            GameScreen::InGame => {
                self.update_game(graphics, dt);
            }
            GameScreen::Paused => {
                self.update_paused(graphics);
            }
            GameScreen::Settings => {
                self.update_settings(graphics);
            }
            GameScreen::About => {
                self.update_about(graphics);
            }
            GameScreen::Visualizer => {
                self.update_visualizer(graphics);
            }
            GameScreen::Ending => {
                self.update_ending(graphics, dt);
            }
        }

        // FPS counter (`?debug` only): real rAF frames over a rolling
        // half-second window, drawn on every screen, top-right.
        if self.debug_enabled {
            self.fps_frames += 1;
            let elapsed = current_time - self.fps_window;
            if elapsed >= 500.0 {
                self.fps_value = self.fps_frames as f32 * 1000.0 / elapsed as f32;
                self.fps_frames = 0;
                self.fps_window = current_time;
            }
            if self.fps_value > 0.0 {
                let text = format!("{:.0} FPS", self.fps_value);
                graphics.draw_text(
                    &text,
                    Vec2::new(graphics.width() - 90.0, graphics.height() - 28.0),
                    18.0,
                    Color::new(0.3, 1.0, 0.5, 0.9),
                );
            }
        }

        // The title-screen car idles under the menu: the baked
        // engine-idle loop starts once audio is unlocked (the same
        // first key/click gesture that lets the context run at all — a
        // browser would ignore anything earlier), keeps looping under
        // the SETTINGS / ABOUT modals over the title, and stops the
        // moment any other screen takes over (game start, ?viz, the
        // credits). Until its bake lands the start call is a silent
        // no-op and simply retries next frame; stop is idempotent.
        let on_title = matches!(
            self.screen,
            GameScreen::LevelSelect | GameScreen::Settings | GameScreen::About
        );
        if on_title {
            // The title has no song: whatever floor / credits track was
            // playing when we got here (QUIT TO MENU, the end of the
            // credits) stops — it used to keep looping under the idle.
            self.audio.stop_music();
            if self.audio_unlocked {
                self.audio.start_engine_idle();
            }
        } else {
            self.audio.stop_engine_idle();
        }

        // Keep the music scheduler fed regardless of screen. (`music`
        // span: note scheduling / node creation for the tracker.) The
        // voice pre-render queue burns down fast on non-gameplay screens
        // (a bake-kick hitch is invisible there) and gently in-game.
        self.audio
            .set_pump_budget(if self.screen == GameScreen::InGame {
                1
            } else {
                6
            });
        let music_span = perf::span("music");
        self.audio.update(current_time / 1000.0);
        drop(music_span);

        // Hand the completed frame to the JS WebGL renderer. The `flush`
        // span measures the whole JS renderer synchronously (frameRender
        // runs inside it); renderer sub-spans nest inside it on the
        // timeline.
        let flush_span = perf::span("flush");
        graphics.flush();
        drop(flush_span);

        // Update input state for next frame
        input::end_frame();
    }
}

#[wasm_bindgen]
pub fn start() -> Result<(), JsValue> {
    // Setup input handlers
    input::setup_input_handlers()?;

    // Initialize graphics
    let graphics = Graphics::new()?;

    // Initialize game state
    let game_state = Rc::new(RefCell::new(GameState::new()));

    // Create game loop closure
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    let window = web_sys::window().ok_or("No window")?;
    let performance = window.performance().ok_or("No performance")?;

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        let current_time = performance.now();
        // FPS CAP: skip the whole frame (no sim, no draw — dt simply
        // accumulates into the next rendered frame) when the display
        // outruns the configured cap. The 0.9 factor keeps a cap equal
        // to the refresh rate from beat-skipping.
        let run = {
            let state = game_state.borrow();
            state.fps_cap == 0
                || current_time - state.last_frame_ms >= 1000.0 / state.fps_cap as f64 * 0.9
        };
        if run {
            perf::frame_start(current_time);
            let mut state = game_state.borrow_mut();
            state.last_frame_ms = current_time;
            state.update(&graphics, current_time);
            perf::frame_end();
        }

        // Schedule next frame
        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));

    request_animation_frame(g.borrow().as_ref().unwrap());

    Ok(())
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    web_sys::window()
        .unwrap()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("Failed to request animation frame");
}
