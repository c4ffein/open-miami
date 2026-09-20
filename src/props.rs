//! The prop sprite library: TOP-DOWN set dressing drawn entirely from the 2D
//! command-stream primitives (no assets, no new dependencies) and animated by
//! the continuous clock, in three FAMILIES ([`PROP_FAMILIES`], contiguous id
//! ranges): DATACENTER — the server floors' racks, switching, cooling, power,
//! storage and hazard furniture (blinking LEDs, spinning roof fans, rising
//! bubbles, a patrolling tape-picker arm) —, OUTDOOR — the night-time gate /
//! parking lot of the planned floor 00 (autonomous cars with their lidar
//! pucks, a charging pad, the main gate's swing arm, a guard booth, lamp
//! posts, road decals, a landed drone, a holo billboard) — and LOBBY — the
//! ground-floor welcome hall (reception desk, turnstiles, scanner arch,
//! benches, holo screens, vending, lockers, the floor logo).
//!
//! Every prop is seen from straight above, matching the game camera: what you
//! draw is the object's top surface. Conventions shared by the set — the
//! machine's front face is the bottom edge (+y), where its status LEDs live;
//! light comes from the top-left, so tall props cast a small drop shadow
//! down-right; screens are edge-on slabs that wash their light across the
//! floor or desk in front of them.
//!
//! Each prop is designed in a local 100x100 box centred on the origin
//! (y down) and drawn through the transform stack, so it can be placed at
//! any size.
//!
//! # Layers
//!
//! A prop is a list of LAYERS ([`PROP_LAYERS`], one [`LayerDef`] each),
//! drawn bottom to top. A layer is drawn in its OWN local frame — origin at
//! its `pivot` (prop coordinates), unrotated — and the driver applies the
//! layer's rotation ([`LayerRot`]: none, a static angle, a spin, a sway or an
//! arbitrary `fn(t) -> rad`) around that pivot. This is what makes props
//! pixelatable per layer: with a pixel size `px` (in design units, so an art
//! pixel scales with the prop) each layer is rasterized in its own pixel-art
//! group (`Graphics::pixel_begin` / `pixel_end`) either
//!
//! * BEFORE its rotation ([`PixelMode::Before`]): the layer is rasterized
//!   unrotated on its own grid and the finished pixel image is rotated as a
//!   whole (a rotated sprite: bodies, lids, panels), or
//! * AFTER it ([`PixelMode::After`]): the group is opened in the parent's
//!   frame and the rotation happens inside it, so the layer is re-rasterized
//!   on the parent's grid every frame (fans, camera heads, needles: the
//!   blades animate through a fixed grid).
//!
//! `px <= 1` draws the layers directly (no groups) and is pixel-for-pixel
//! the pre-layer look. The saved per-prop `px` and per-layer modes live in
//! `props/props.json` (compiled to `src/props_data.rs` by `make gen-props`,
//! see `docs/PROPS_FORMAT.md`); [`draw_prop`] uses them, [`draw_prop_ex`]
//! takes them from the caller (the `?viz` PROPS page edits and saves them).

use crate::props_data::PROP_SETTINGS;
use std::f32::consts::{PI, TAU};

/// Display names, indexed by prop id (the order of the library).
pub const PROP_NAMES: [&str; 60] = [
    "RACK / CLOSED",
    "RACK / OPEN",
    "RACK / BURNT",
    "BLADE STACK",
    "CORE SWITCH",
    "CABLE JUNCTION",
    "OPERATOR DESK",
    "CONTROL CONSOLE",
    "HOLO TABLE",
    "CRAC COOLER",
    "FLOOR VENT",
    "EXHAUST FAN",
    "COOLANT TANK",
    "PIPE RUN",
    "UPS CABINET",
    "GENERATOR",
    "CABLE TRAY",
    "CABLE COIL",
    "TAPE LIBRARY",
    "SUPPLY CRATE",
    "SECURITY CAM",
    "FIRE SUPPRESSOR",
    "HAZARD PAD",
    "UPLINK OBELISK",
    // ---- OUTDOOR: the gate / parking lot (floor 00, night, neon) ---------
    "CAR / POD",
    "CAR / SEDAN",
    "CAR / OPEN",
    "DELIVERY VAN",
    "CHARGE PAD",
    "CAR / CHARGING",
    "MAIN GATE",
    "GUARD BOOTH",
    "BOLLARDS",
    "PLANTER",
    "LAMP POST",
    "EV BAY",
    "CROSSWALK",
    "DRONE PAD",
    "SCOOTER RACK",
    "DRAIN GRATE",
    "HOLO BILLBOARD",
    "DUMPSTER",
    // ---- LOBBY: the ground-floor welcome hall -----------------------------
    "RECEPTION DESK",
    "TURNSTILES",
    "SCANNER ARCH",
    "BENCH / LONG",
    "BENCH / SHORT",
    "POTTED PLANT",
    "LOBBY HOLO",
    "DIRECTORY TOTEM",
    "VENDING MACHINE",
    "COFFEE CORNER",
    "CHARGE LOCKERS",
    "FLOOR LOGO",
    "CALL PANEL",
    "VELVET ROPE",
    "EXTINGUISHER",
    "CREDIT KIOSK",
    "WALL CLOCK",
    "WELCOME MAT",
];

/// Number of props in the library.
pub const PROP_COUNT: usize = PROP_NAMES.len();

/// The prop FAMILIES (the `?viz` PROPS gallery pages): display name and the
/// id of the family's first prop. Families are contiguous id ranges, in
/// library order — DATACENTER (the server floors), OUTDOOR (the gate /
/// parking lot of the planned floor 00) and LOBBY (the ground-floor welcome
/// hall).
pub const PROP_FAMILIES: [(&str, usize); 3] = [("DATACENTER", 0), ("OUTDOOR", 24), ("LOBBY", 42)];

/// The id range of family `family` (see [`PROP_FAMILIES`]).
pub fn family_range(family: usize) -> std::ops::Range<usize> {
    let f = family % PROP_FAMILIES.len();
    let start = PROP_FAMILIES[f].1;
    let end = PROP_FAMILIES.get(f + 1).map(|n| n.1).unwrap_or(PROP_COUNT);
    start..end
}

/// The family of prop `kind`.
pub fn prop_family(kind: usize) -> usize {
    let kind = kind % PROP_COUNT;
    (0..PROP_FAMILIES.len())
        .rev()
        .find(|&f| PROP_FAMILIES[f].1 <= kind)
        .unwrap_or(0)
}

/// Most props in any one family (sizes the gallery grid).
pub fn largest_family() -> usize {
    (0..PROP_FAMILIES.len())
        .map(|f| family_range(f).len())
        .max()
        .unwrap_or(0)
}

/// Upper bound on layers per prop (the per-layer settings arrays / the
/// visibility bitmask are sized by it).
pub const MAX_LAYERS: usize = 8;

/// Largest saved / selectable art-pixel size (design units; 1 = off).
pub const MAX_PX: u32 = 10;

/// When a layer's pixel-art group is rasterized relative to its rotation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelMode {
    /// Rasterize the layer unrotated on its own grid, then rotate the pixel
    /// image as a whole (a rotated sprite).
    Before,
    /// Rotate first, then rasterize on the parent's grid (re-rasterized every
    /// frame as it turns).
    After,
}

impl PixelMode {
    /// The JSON id (`"before"` / `"after"`).
    pub fn id(self) -> &'static str {
        match self {
            PixelMode::Before => "before",
            PixelMode::After => "after",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "before" => Some(PixelMode::Before),
            "after" => Some(PixelMode::After),
            _ => None,
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            PixelMode::Before => PixelMode::After,
            PixelMode::After => PixelMode::Before,
        }
    }
}

/// How a layer rotates around its pivot, as a function of the clock.
#[derive(Clone, Copy)]
pub enum LayerRot {
    /// Fixed to the parent.
    None,
    /// A constant angle (degrees).
    Static(f32),
    /// Continuous spin at `hz` revolutions per second (negative = the other
    /// way).
    Spin { hz: f32 },
    /// Oscillation of ± `deg` degrees at `hz` cycles per second.
    Sway { deg: f32, hz: f32 },
    /// Anything else: the angle in radians from the clock.
    Anim(fn(f32) -> f32),
}

impl LayerRot {
    /// The layer's angle (radians) at clock `t`.
    pub fn angle(&self, t: f32) -> f32 {
        match *self {
            LayerRot::None => 0.0,
            LayerRot::Static(deg) => deg * (PI / 180.0),
            LayerRot::Spin { hz } => t * hz * TAU,
            LayerRot::Sway { deg, hz } => deg * (PI / 180.0) * (t * hz * TAU).sin(),
            LayerRot::Anim(f) => f(t),
        }
    }

    /// Short human description for the inspector.
    pub fn label(&self) -> String {
        match *self {
            LayerRot::None => "fixed".to_string(),
            LayerRot::Static(deg) => format!("static {:+.0}°", deg),
            LayerRot::Spin { hz } => format!("spin {:+.2} Hz", hz),
            LayerRot::Sway { deg, hz } => format!("sway ±{:.0}° {:.2} Hz", deg, hz),
            LayerRot::Anim(_) => "anim".to_string(),
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, LayerRot::None)
    }
}

/// One drawable layer of a prop.
#[derive(Clone, Copy)]
pub struct LayerDef {
    /// Unique within the prop; the key in `props/props.json`.
    pub name: &'static str,
    /// The layer's origin / rotation centre, in prop coordinates.
    pub pivot: (f32, f32),
    /// Local AABB `(x, y, w, h)` of what the layer draws, in ITS frame
    /// (relative to the pivot, unrotated): sizes its pixel group.
    pub bounds: (f32, f32, f32, f32),
    pub rot: LayerRot,
    /// The default pixel mode (`props.json` may override it).
    pub pixel: PixelMode,
}

/// Degrees per radian, for the static angles the props were designed in.
const DEG: f32 = 180.0 / PI;

const fn layer(
    name: &'static str,
    pivot: (f32, f32),
    bounds: (f32, f32, f32, f32),
    rot: LayerRot,
    pixel: PixelMode,
) -> LayerDef {
    LayerDef {
        name,
        pivot,
        bounds,
        rot,
        pixel,
    }
}

use PixelMode::{After, Before};

// ---- the shared animation curves of the OUTDOOR / LOBBY layers -----------

/// Smooth 0..1 ramp of `x` over `a..b` (constant outside).
fn smooth(x: f32, a: f32, b: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The main gate's swing arm: closed across the lane, then a slow 90° swing
/// open along the lane (radians, negative = towards -y), a hold, and back —
/// a 14 s cycle.
fn gate_angle(t: f32) -> f32 {
    let c = t.rem_euclid(14.0);
    let open = smooth(c, 6.0, 8.2) - smooth(c, 12.0, 13.8);
    -open * (PI / 2.0)
}

/// The turnstile's free lane: the arm swings 75° with the walker (towards
/// -y) and drops back — a 6 s cycle, most of it closed.
fn turnstile_angle(t: f32) -> f32 {
    let c = t.rem_euclid(6.0);
    let open = smooth(c, 3.0, 3.6) - smooth(c, 5.2, 5.8);
    open * (75.0 * PI / 180.0)
}

/// A landed drone's rotor idling: still, with a short shiver now and then
/// (each rotor on its own phase).
fn rotor_twitch(t: f32, phase: f32) -> f32 {
    if (t * 0.45 + phase).sin() > 0.82 {
        0.55 * (t * 31.0 + phase).sin()
    } else {
        0.0
    }
}

mod layers;
pub use layers::PROP_LAYERS;

/// The layers of prop `kind`.
pub fn prop_layers(kind: usize) -> &'static [LayerDef] {
    PROP_LAYERS[kind % PROP_COUNT]
}

/// The JSON id of prop `kind`: `PROP_NAMES` lower-cased, every run of
/// non-alphanumerics collapsed to one `_` (`"RACK / CLOSED"` -> `rack_closed`).
pub fn prop_kind_id(kind: usize) -> String {
    let mut out = String::new();
    for ch in PROP_NAMES[kind % PROP_COUNT].chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    out.trim_end_matches('_').to_string()
}

// ---- persisted settings (generated: src/props_data.rs) --------------------

/// A layer's saved pixel mode (`props/props.json` -> `PROP_SETTINGS`).
pub struct LayerSetting {
    pub name: &'static str,
    pub pixel: PixelMode,
}

/// A prop's saved settings: its art-pixel size (design units, 1 = off) and
/// the layers whose pixel mode differs from / is pinned against the
/// [`LayerDef`] default. Unknown layer names are ignored, missing ones use
/// the default.
pub struct PropSettings {
    pub kind: &'static str,
    pub px: u32,
    pub layers: &'static [LayerSetting],
}

/// The saved art-pixel size of prop `kind` (1 = off).
pub fn prop_px(kind: usize) -> u32 {
    PROP_SETTINGS[kind % PROP_COUNT].px.clamp(1, MAX_PX)
}

/// The per-layer pixel modes of prop `kind`: the [`LayerDef`] defaults with
/// the saved overrides applied (entries past the prop's layer count are
/// `Before`).
pub fn prop_modes(kind: usize) -> [PixelMode; MAX_LAYERS] {
    let kind = kind % PROP_COUNT;
    let mut modes = [Before; MAX_LAYERS];
    for (i, l) in PROP_LAYERS[kind].iter().enumerate() {
        modes[i] = PROP_SETTINGS[kind]
            .layers
            .iter()
            .find(|s| s.name == l.name)
            .map(|s| s.pixel)
            .unwrap_or(l.pixel);
    }
    modes
}

/// Serialize per-prop `(px, modes)` (one entry per prop, in id order) to the
/// `props/props.json` document (`docs/PROPS_FORMAT.md`), in the checked-in
/// formatting: one prop per line.
pub fn settings_json(props: &[(u32, [PixelMode; MAX_LAYERS])]) -> String {
    let mut out = String::from("{\n  \"props\": [\n");
    for (kind, (px, modes)) in props.iter().enumerate().take(PROP_COUNT) {
        let layers: Vec<String> = PROP_LAYERS[kind]
            .iter()
            .enumerate()
            .map(|(i, l)| {
                format!(
                    "{{\"name\": \"{}\", \"pixel\": \"{}\"}}",
                    l.name,
                    modes[i].id()
                )
            })
            .collect();
        out.push_str(&format!(
            "    {{\"kind\": \"{}\", \"px\": {}, \"layers\": [{}]}}",
            prop_kind_id(kind),
            (*px).clamp(1, MAX_PX),
            layers.join(", ")
        ));
        out.push_str(if kind + 1 < PROP_COUNT.min(props.len()) {
            ",\n"
        } else {
            "\n"
        });
    }
    out.push_str("  ]\n}\n");
    out
}

/// Per-draw options of [`draw_prop_ex`].
#[derive(Clone, Copy)]
pub struct PropDrawOpts {
    /// Bit `i` set = layer `i` is drawn.
    pub visible: u32,
    /// Pixel mode per layer (only used with `px >= 2`).
    pub modes: [PixelMode; MAX_LAYERS],
}

impl PropDrawOpts {
    /// Every layer visible, the saved modes of prop `kind`.
    pub fn saved(kind: usize) -> Self {
        PropDrawOpts {
            visible: u32::MAX,
            modes: prop_modes(kind),
        }
    }
}

/// The on-screen size to draw a prop at so its art pixels map to an INTEGER
/// number of device pixels: the largest `k` art texels -> `k` px that fits in
/// `size_px` (`100 / px` art texels across the prop). NEAREST upscaling then
/// never straddles texel rows unevenly, and every layer's group snaps to the
/// same device grid. `px <= 1`, or a prop smaller than one px per texel,
/// returns `size_px` unchanged.
pub fn snap_size(size_px: f32, px: u32) -> f32 {
    if px <= 1 {
        return size_px;
    }
    let texels = 100.0 / px as f32;
    let k = (size_px / texels).floor();
    if k >= 1.0 {
        texels * k
    } else {
        size_px
    }
}

/// The pixel group box for a layer AABB: origin snapped down to the layer
/// frame's `px` grid and the far edge up, so layers sharing a pivot share one
/// grid (and edges designed on multiples of `px` land on texel edges).
pub fn snap_box(bounds: (f32, f32, f32, f32), px: f32) -> (f32, f32, f32, f32) {
    let (x, y, w, h) = bounds;
    let gx = (x / px).floor() * px;
    let gy = (y / px).floor() * px;
    let gw = ((x + w) / px).ceil() * px - gx;
    let gh = ((y + h) / px).ceil() * px - gy;
    (gx, gy, gw, gh)
}

/// The square group box that holds `bounds` at ANY rotation about the
/// origin (for `After` layers that turn): radius = the farthest corner.
pub fn rot_box(bounds: (f32, f32, f32, f32), px: f32) -> (f32, f32, f32, f32) {
    let (x, y, w, h) = bounds;
    let r = [(x, y), (x + w, y), (x, y + h), (x + w, y + h)]
        .iter()
        .map(|&(cx, cy)| (cx * cx + cy * cy).sqrt())
        .fold(0.0f32, f32::max);
    snap_box((-r, -r, 2.0 * r, 2.0 * r), px)
}

mod draw;
pub use draw::{draw_prop, draw_prop_ex, draw_prop_layer};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prop_names_are_present_and_unique() {
        assert_eq!(PROP_COUNT, PROP_NAMES.len());
        for (i, a) in PROP_NAMES.iter().enumerate() {
            assert!(!a.is_empty());
            for b in PROP_NAMES.iter().skip(i + 1) {
                assert_ne!(a, b, "duplicate prop name");
            }
        }
    }

    /// The families partition the library into contiguous id ranges, in
    /// order, and every prop maps back to the family whose range holds it.
    #[test]
    fn families_partition_the_library() {
        assert_eq!(PROP_FAMILIES[0], ("DATACENTER", 0));
        assert_eq!(
            family_range(0),
            0..24,
            "the datacenter set keeps its 24 ids"
        );
        let mut next = 0;
        for f in 0..PROP_FAMILIES.len() {
            let r = family_range(f);
            assert_eq!(
                r.start, next,
                "family {f} does not start where the last ended"
            );
            assert!(!r.is_empty(), "family {f} is empty");
            for k in r.clone() {
                assert_eq!(prop_family(k), f, "prop {k}");
            }
            next = r.end;
        }
        assert_eq!(next, PROP_COUNT);
        assert!(largest_family() >= 24 && largest_family() <= 4 * 8);
        assert_eq!(prop_kind_id(PROP_FAMILIES[1].1), "car_pod");
        assert_eq!(prop_kind_id(PROP_FAMILIES[2].1), "reception_desk");
    }

    #[test]
    fn kind_ids_are_snake_case_and_unique() {
        let ids: Vec<String> = (0..PROP_COUNT).map(prop_kind_id).collect();
        assert_eq!(ids[0], "rack_closed");
        assert_eq!(ids[9], "crac_cooler");
        for (i, a) in ids.iter().enumerate() {
            assert!(a
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'));
            assert!(!a.starts_with('_') && !a.ends_with('_'));
            for b in ids.iter().skip(i + 1) {
                assert_ne!(a, b, "duplicate prop id");
            }
        }
    }

    #[test]
    fn every_prop_has_layers_with_unique_names_and_sane_bounds() {
        for (kind, layers) in PROP_LAYERS.iter().enumerate() {
            assert!(!layers.is_empty(), "prop {kind} has no layers");
            assert!(
                layers.len() <= MAX_LAYERS,
                "prop {kind} has too many layers"
            );
            for (i, l) in layers.iter().enumerate() {
                assert!(!l.name.is_empty());
                assert!(
                    l.bounds.2 > 0.0 && l.bounds.3 > 0.0,
                    "prop {kind} layer {i}"
                );
                for m in layers.iter().skip(i + 1) {
                    assert_ne!(l.name, m.name, "prop {kind}: duplicate layer name");
                }
            }
        }
    }

    /// The generated settings (props/props.json -> props_data.rs) must refer
    /// to real props and layers, in library order.
    #[test]
    fn generated_settings_match_the_library() {
        for (kind, s) in PROP_SETTINGS.iter().enumerate() {
            assert_eq!(
                s.kind,
                prop_kind_id(kind),
                "props_data.rs is out of order/date"
            );
            assert!((1..=MAX_PX).contains(&s.px), "{}: px out of range", s.kind);
            for l in s.layers {
                assert!(
                    PROP_LAYERS[kind].iter().any(|d| d.name == l.name),
                    "{}: unknown layer '{}' in props.json (run `make gen-props` after renaming)",
                    s.kind,
                    l.name
                );
            }
        }
    }

    #[test]
    fn settings_json_round_trips_the_defaults() {
        let entries: Vec<(u32, [PixelMode; MAX_LAYERS])> = (0..PROP_COUNT)
            .map(|k| (prop_px(k), prop_modes(k)))
            .collect();
        let json = settings_json(&entries);
        assert!(json.starts_with("{\n  \"props\": [\n"));
        assert!(json.contains("\"kind\": \"security_cam\""));
        assert_eq!(json.matches("\"kind\"").count(), PROP_COUNT);
        assert_eq!(
            json.matches("\"name\"").count(),
            PROP_LAYERS.iter().map(|l| l.len()).sum::<usize>()
        );
    }

    #[test]
    fn rotations_and_boxes() {
        assert_eq!(LayerRot::None.angle(3.0), 0.0);
        assert!((LayerRot::Static(90.0).angle(0.0) - PI / 2.0).abs() < 1e-6);
        assert!((LayerRot::Spin { hz: 1.0 }.angle(0.25) - PI / 2.0).abs() < 1e-6);
        assert_eq!(
            snap_box((-35.0, 37.0, 70.0, 8.0), 4.0),
            (-36.0, 36.0, 72.0, 12.0)
        );
        let (x, y, w, h) = rot_box((-14.0, -14.0, 28.0, 28.0), 4.0);
        assert_eq!((x, y), (-20.0, -20.0));
        assert_eq!((w, h), (40.0, 40.0));
        assert_eq!(snap_size(75.0, 4), 75.0); // 25 texels x 3
        assert_eq!(snap_size(368.0, 4), 350.0); // 25 texels x 14
        assert_eq!(snap_size(20.0, 4), 20.0); // < 1 px per texel: unchanged
        assert_eq!(snap_size(368.0, 1), 368.0);
        assert_eq!(PixelMode::from_id("after"), Some(After));
        assert_eq!(PixelMode::from_id("x"), None);
        assert_eq!(Before.toggled(), After);
    }
}
