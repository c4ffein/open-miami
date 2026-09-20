//! The LOBBY family: layer tables of props 42..59 (index = prop id - 42).

use super::*;

pub(super) const LAYERS: [&[LayerDef]; 18] = [
    // ========================= LOBBY: the welcome hall =========================
    // 42 RECEPTION DESK
    &[
        layer(
            "shadow",
            (0.0, 0.0),
            (-40.0, -10.0, 84.0, 24.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "wing l",
            (-40.0, 0.0),
            (-26.0, -10.0, 30.0, 24.0),
            LayerRot::Static(-25.0),
            Before,
        ),
        layer(
            "wing r",
            (40.0, 0.0),
            (-4.0, -10.0, 34.0, 24.0),
            LayerRot::Static(25.0),
            Before,
        ),
        layer(
            "desk",
            (0.0, 0.0),
            (-40.0, -18.0, 80.0, 30.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "terminal",
            (0.0, -4.0),
            (-21.0, -26.0, 42.0, 32.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "chair",
            (0.0, 0.0),
            (-16.0, -42.0, 34.0, 32.0),
            LayerRot::None,
            After,
        ),
    ],
    // 43 TURNSTILES
    &[
        layer(
            "floor",
            (0.0, 0.0),
            (-46.0, -30.0, 92.0, 62.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "housings",
            (0.0, 0.0),
            (-46.0, -20.0, 96.0, 48.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "arm l",
            (-34.0, 0.0),
            (-4.0, -4.0, 34.0, 8.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "arm r",
            (34.0, 0.0),
            (-30.0, -4.0, 34.0, 8.0),
            LayerRot::Anim(turnstile_angle),
            After,
        ),
        layer(
            "leds",
            (0.0, 0.0),
            (-44.0, 12.0, 88.0, 6.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 44 SCANNER ARCH
    &[
        layer(
            "mat",
            (0.0, 0.0),
            (-40.0, -26.0, 80.0, 52.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "arch",
            (0.0, 0.0),
            (-36.0, -20.0, 80.0, 50.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "sweep",
            (0.0, 0.0),
            (-24.0, -19.0, 48.0, 38.0),
            LayerRot::None,
            After,
        ),
        layer(
            "leds",
            (0.0, 0.0),
            (-34.0, -19.0, 68.0, 8.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 45 BENCH / LONG
    &[
        layer(
            "bench",
            (0.0, 0.0),
            (-44.0, -20.0, 92.0, 42.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "items",
            (0.0, 0.0),
            (-30.0, -8.0, 60.0, 16.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 46 BENCH / SHORT
    &[
        layer(
            "bench",
            (0.0, 0.0),
            (-30.0, -20.0, 64.0, 42.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "items",
            (0.0, 0.0),
            (-20.0, -10.0, 40.0, 20.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 47 POTTED PLANT
    &[
        layer(
            "pot",
            (0.0, 0.0),
            (-20.0, -20.0, 44.0, 44.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "leaves",
            (0.0, 0.0),
            (-31.0, -31.0, 62.0, 62.0),
            LayerRot::Sway { deg: 3.0, hz: 0.2 },
            After,
        ),
        layer(
            "tag",
            (0.0, 0.0),
            (8.0, 4.0, 12.0, 14.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 48 LOBBY HOLO
    &[
        layer(
            "wash",
            (0.0, 0.0),
            (-50.0, -28.0, 100.0, 72.0),
            LayerRot::None,
            After,
        ),
        layer(
            "slab",
            (0.0, 0.0),
            (-44.0, -44.0, 88.0, 18.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "glyphs",
            (0.0, 0.0),
            (-44.0, -37.0, 88.0, 10.0),
            LayerRot::None,
            After,
        ),
    ],
    // 49 DIRECTORY TOTEM
    &[
        layer(
            "body",
            (0.0, 0.0),
            (-10.0, -22.0, 25.0, 49.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "screen",
            (0.0, 0.0),
            (-8.0, -18.0, 16.0, 30.0),
            LayerRot::None,
            After,
        ),
        layer(
            "wash",
            (0.0, 0.0),
            (-16.0, 20.0, 32.0, 18.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 50 VENDING MACHINE
    &[
        layer(
            "body",
            (0.0, 0.0),
            (-24.0, -20.0, 53.0, 45.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "front",
            (0.0, 0.0),
            (-22.0, 13.0, 44.0, 8.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "wash",
            (0.0, 0.0),
            (-30.0, 20.0, 60.0, 20.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 51 COFFEE CORNER
    &[
        layer(
            "counter",
            (0.0, 0.0),
            (-35.0, -14.0, 74.0, 32.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "machine",
            (0.0, 0.0),
            (-30.0, -12.0, 28.0, 26.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "cups",
            (0.0, 0.0),
            (0.0, -12.0, 34.0, 24.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "steam",
            (-17.0, -12.0),
            (-8.0, -20.0, 16.0, 22.0),
            LayerRot::None,
            After,
        ),
    ],
    // 52 CHARGE LOCKERS
    &[
        layer(
            "cabinet",
            (0.0, 0.0),
            (-40.0, -16.0, 84.0, 36.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "leds",
            (0.0, 0.0),
            (-40.0, 10.0, 80.0, 8.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "cable",
            (0.0, 0.0),
            (10.0, 16.0, 30.0, 22.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 53 FLOOR LOGO
    &[
        layer(
            "inlay",
            (0.0, 0.0),
            (-40.0, -40.0, 80.0, 80.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "mark",
            (0.0, 0.0),
            (-13.0, -13.0, 26.0, 26.0),
            LayerRot::Static(45.0),
            Before,
        ),
        layer(
            "seams",
            (0.0, 0.0),
            (-26.0, -26.0, 52.0, 52.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 54 CALL PANEL
    &[
        layer(
            "wall",
            (0.0, 0.0),
            (-30.0, -46.0, 60.0, 14.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "panel",
            (0.0, 0.0),
            (11.0, -35.0, 16.0, 12.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "arrows",
            (0.0, 0.0),
            (10.0, -34.0, 18.0, 20.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 55 VELVET ROPE
    &[
        layer(
            "shadow",
            (0.0, 0.0),
            (-34.0, -4.0, 80.0, 18.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "rope",
            (0.0, 0.0),
            (-36.0, -3.0, 72.0, 12.0),
            LayerRot::None,
            After,
        ),
        layer(
            "posts",
            (0.0, 0.0),
            (-44.0, -8.0, 92.0, 16.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 56 EXTINGUISHER
    &[
        layer(
            "mount",
            (0.0, 0.0),
            (-16.0, -46.0, 32.0, 14.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "tank",
            (0.0, 0.0),
            (-12.0, -38.0, 28.0, 28.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "sign",
            (0.0, 0.0),
            (-10.0, -6.0, 20.0, 20.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 57 CREDIT KIOSK
    &[
        layer(
            "body",
            (0.0, 0.0),
            (-15.0, -20.0, 35.0, 45.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "screen",
            (0.0, 0.0),
            (-11.0, -16.0, 22.0, 14.0),
            LayerRot::None,
            After,
        ),
        layer(
            "wash",
            (0.0, 0.0),
            (-21.0, 20.0, 42.0, 16.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 58 WALL CLOCK
    &[
        layer(
            "face",
            (0.0, 0.0),
            (-36.0, -36.0, 72.0, 72.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "hour",
            (0.0, 0.0),
            (-3.0, -20.0, 6.0, 24.0),
            LayerRot::Static(-50.0),
            After,
        ),
        layer(
            "minute",
            (0.0, 0.0),
            (-2.0, -28.0, 4.0, 32.0),
            LayerRot::Spin { hz: 1.0 / 40.0 },
            After,
        ),
        layer(
            "second",
            (0.0, 0.0),
            (-1.0, -30.0, 2.0, 36.0),
            LayerRot::Spin { hz: 1.0 / 8.0 },
            After,
        ),
    ],
    // 59 WELCOME MAT
    &[
        layer(
            "mat",
            (0.0, 0.0),
            (-35.0, -20.0, 70.0, 40.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "pattern",
            (0.0, 0.0),
            (-31.0, -16.0, 62.0, 32.0),
            LayerRot::None,
            Before,
        ),
    ],
];
