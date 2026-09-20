//! The DATACENTER family: layer tables of props 0..23 (index = prop id - 0).

use super::*;

pub(super) const LAYERS: [&[LayerDef]; 24] = [
    // 0 RACK / CLOSED
    &[
        layer(
            "body",
            (0.0, 0.0),
            (-36.0, -50.0, 76.0, 100.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "fan a",
            (0.0, -15.0),
            (-14.0, -14.0, 28.0, 28.0),
            LayerRot::Spin { hz: 3.4 / TAU },
            After,
        ),
        layer(
            "fan b",
            (0.0, 15.0),
            (-14.0, -14.0, 28.0, 28.0),
            LayerRot::Anim(|t| (t + 0.4) * -2.9),
            After,
        ),
        layer(
            "leds",
            (0.0, 0.0),
            (-35.0, 37.0, 70.0, 8.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 1 RACK / OPEN
    &[
        layer(
            "chassis",
            (0.0, 0.0),
            (-36.0, -46.0, 76.0, 96.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "fan",
            (0.0, -12.0),
            (-11.0, -11.0, 22.0, 22.0),
            LayerRot::Spin { hz: 4.6 / TAU },
            After,
        ),
        layer(
            "leds",
            (0.0, 0.0),
            (-26.0, 24.0, 3.0, 13.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 2 RACK / BURNT
    &[
        layer(
            "lid",
            (36.0, 8.0),
            (-9.0, -28.0, 18.0, 56.0),
            LayerRot::Static(0.25 * DEG),
            Before,
        ),
        layer(
            "hull",
            (0.0, 0.0),
            (-41.0, -46.0, 70.0, 96.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "sparks",
            (0.0, 0.0),
            (-25.0, -33.0, 40.0, 50.0),
            LayerRot::None,
            After,
        ),
    ],
    // 3 BLADE STACK
    &[
        layer(
            "enclosure",
            (0.0, 0.0),
            (-41.0, -43.0, 86.0, 90.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "leds",
            (0.0, 0.0),
            (-40.0, 34.0, 80.0, 8.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 4 CORE SWITCH
    &[
        layer(
            "chassis",
            (0.0, 0.0),
            (-46.0, -23.0, 96.0, 46.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "ports",
            (0.0, 0.0),
            (-45.0, 12.0, 90.0, 6.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "cables",
            (0.0, 0.0),
            (-40.0, 18.0, 90.0, 26.0),
            LayerRot::None,
            After,
        ),
    ],
    // 5 CABLE JUNCTION
    &[
        layer(
            "boxes",
            (0.0, 0.0),
            (-46.0, -31.0, 96.0, 66.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "cables",
            (0.0, 0.0),
            (-28.0, -27.0, 56.0, 54.0),
            LayerRot::None,
            After,
        ),
    ],
    // 6 OPERATOR DESK
    &[
        layer(
            "desk",
            (0.0, 0.0),
            (-46.0, -29.0, 96.0, 58.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "paper a",
            (-33.0, 4.0),
            (-8.0, -10.0, 16.0, 20.0),
            LayerRot::Static(-0.2 * DEG),
            Before,
        ),
        layer(
            "paper b",
            (-30.0, 7.0),
            (-8.0, -10.0, 16.0, 20.0),
            LayerRot::Static(0.15 * DEG),
            Before,
        ),
        layer(
            "mug",
            (0.0, 0.0),
            (30.0, 8.0, 15.0, 12.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 7 CONTROL CONSOLE
    &[
        layer(
            "shadow",
            (0.0, 0.0),
            (-30.0, -30.0, 68.0, 26.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "wing l",
            (-40.0, -16.0),
            (-16.0, -12.0, 32.0, 24.0),
            LayerRot::Static(0.55 * DEG),
            Before,
        ),
        layer(
            "wing r",
            (40.0, -16.0),
            (-16.0, -12.0, 32.0, 24.0),
            LayerRot::Static(-0.55 * DEG),
            Before,
        ),
        layer(
            "desk",
            (0.0, 0.0),
            (-34.0, -34.0, 68.0, 26.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "screen l",
            (-27.0, -24.0),
            (-16.0, -3.0, 32.0, 26.0),
            LayerRot::Static(0.5 * DEG),
            Before,
        ),
        layer(
            "screen c",
            (0.0, -24.0),
            (-16.0, -3.0, 32.0, 26.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "screen r",
            (27.0, -24.0),
            (-16.0, -3.0, 32.0, 26.0),
            LayerRot::Static(-0.5 * DEG),
            Before,
        ),
        layer(
            "chair",
            (0.0, 0.0),
            (-16.0, 12.0, 34.0, 30.0),
            LayerRot::None,
            After,
        ),
    ],
    // 8 HOLO TABLE
    &[
        layer(
            "pedestal",
            (0.0, 0.0),
            (-28.0, -14.0, 60.0, 60.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "hologram",
            (0.0, 0.0),
            (-27.0, -29.0, 54.0, 42.0),
            LayerRot::None,
            After,
        ),
    ],
    // 9 CRAC COOLER
    &[
        layer(
            "housing",
            (0.0, 0.0),
            (-41.0, -46.0, 86.0, 96.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "blower",
            (0.0, 10.0),
            (-27.0, -27.0, 54.0, 54.0),
            LayerRot::Spin { hz: 4.0 / TAU },
            After,
        ),
        layer(
            "led",
            (0.0, 0.0),
            (-40.0, 39.0, 80.0, 6.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 10 FLOOR VENT
    &[
        layer(
            "grille",
            (0.0, 0.0),
            (-43.0, -43.0, 86.0, 86.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "airflow",
            (0.0, 0.0),
            (-39.0, -39.0, 78.0, 78.0),
            LayerRot::None,
            After,
        ),
    ],
    // 11 EXHAUST FAN
    &[
        layer(
            "duct",
            (0.0, 0.0),
            (-43.0, -43.0, 86.0, 86.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "blades",
            (0.0, 0.0),
            (-33.0, -33.0, 66.0, 66.0),
            LayerRot::Spin { hz: 3.2 / TAU },
            After,
        ),
        layer(
            "guard",
            (0.0, 0.0),
            (-37.0, -37.0, 74.0, 74.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 12 COOLANT TANK
    &[
        layer(
            "tank",
            (0.0, 0.0),
            (-39.0, -39.0, 82.0, 82.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "bubbles",
            (0.0, 0.0),
            (-22.0, -24.0, 44.0, 48.0),
            LayerRot::None,
            After,
        ),
        layer(
            "hatch",
            (0.0, 0.0),
            (-10.0, -10.0, 20.0, 20.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 13 PIPE RUN
    &[
        layer(
            "pipes",
            (0.0, 0.0),
            (-50.0, -23.0, 100.0, 50.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "valve",
            (-34.0, 13.0),
            (-10.0, -10.0, 20.0, 20.0),
            LayerRot::Sway {
                deg: 0.3 * DEG,
                hz: 0.6 / TAU,
            },
            After,
        ),
    ],
    // 14 UPS CABINET
    &[
        layer(
            "cabinet",
            (0.0, 0.0),
            (-31.0, -50.0, 66.0, 100.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "bolt",
            (0.0, 0.0),
            (-6.0, 14.0, 12.0, 20.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "charge",
            (0.0, 0.0),
            (-30.0, 37.0, 60.0, 8.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 15 GENERATOR
    &[
        layer(
            "block",
            (0.0, 0.0),
            (-46.0, -26.0, 96.0, 70.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "smoke",
            (-31.0, -36.0),
            (-21.0, -21.0, 42.0, 42.0),
            LayerRot::None,
            After,
        ),
        layer(
            "stack",
            (0.0, 0.0),
            (-38.0, -43.0, 14.0, 21.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "gauge",
            (-12.0, 8.0),
            (-7.0, -7.0, 14.0, 14.0),
            LayerRot::Anim(|t| -1.9 + 0.12 * (t * 9.0).sin()),
            After,
        ),
    ],
    // 16 CABLE TRAY
    &[
        layer(
            "tray",
            (0.0, 0.0),
            (-48.0, -24.0, 96.0, 48.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "cables",
            (0.0, 0.0),
            (-47.0, -17.0, 94.0, 36.0),
            LayerRot::None,
            After,
        ),
    ],
    // 17 CABLE COIL
    &[
        layer(
            "coil",
            (0.0, 0.0),
            (-34.0, -34.0, 72.0, 72.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "glints",
            (0.0, 0.0),
            (-30.0, -30.0, 60.0, 60.0),
            LayerRot::Spin { hz: 0.4 / TAU },
            After,
        ),
        layer(
            "lead",
            (0.0, 0.0),
            (26.0, 12.0, 23.0, 21.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 18 TAPE LIBRARY
    &[
        layer(
            "chassis",
            (0.0, 0.0),
            (-43.0, -46.0, 90.0, 96.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "picker",
            (0.0, 0.0),
            (-24.0, -37.0, 48.0, 74.0),
            LayerRot::None,
            After,
        ),
    ],
    // 19 SUPPLY CRATE
    &[
        layer(
            "crate",
            (0.0, 0.0),
            (-39.0, -33.0, 82.0, 70.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "stencil",
            (0.0, 0.0),
            (-24.0, 10.0, 50.0, 16.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 20 SECURITY CAM
    &[
        layer(
            "mount",
            (0.0, 0.0),
            (-17.0, -49.0, 34.0, 12.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "cone",
            (0.0, -30.0),
            (-20.0, 0.0, 40.0, 65.0),
            LayerRot::Sway {
                deg: 0.35 * DEG,
                hz: 0.5 / TAU,
            },
            After,
        ),
        layer(
            "head",
            (0.0, -30.0),
            (-8.0, -7.0, 16.0, 31.0),
            LayerRot::Sway {
                deg: 0.35 * DEG,
                hz: 0.5 / TAU,
            },
            After,
        ),
    ],
    // 21 FIRE SUPPRESSOR
    &[
        layer(
            "manifold",
            (0.0, 0.0),
            (-31.0, -45.0, 62.0, 34.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "tanks",
            (0.0, 0.0),
            (-36.0, -10.0, 76.0, 40.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "wheel l",
            (-18.0, 8.0),
            (-9.0, -9.0, 18.0, 18.0),
            LayerRot::Anim(|t| 0.25 * (t * 0.5).sin()),
            After,
        ),
        layer(
            "wheel r",
            (18.0, 8.0),
            (-9.0, -9.0, 18.0, 18.0),
            LayerRot::Anim(|t| 0.25 * (t * 0.8).sin() + 1.0),
            After,
        ),
        layer(
            "tag",
            (0.0, 0.0),
            (-4.0, 32.0, 8.0, 10.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 22 HAZARD PAD
    &[
        layer(
            "pad",
            (0.0, 0.0),
            (-44.0, -44.0, 88.0, 88.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "sign",
            (0.0, 0.0),
            (-18.0, -20.0, 36.0, 34.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 23 UPLINK OBELISK
    &[
        layer(
            "aura",
            (0.0, 0.0),
            (-44.0, -44.0, 88.0, 88.0),
            LayerRot::None,
            After,
        ),
        layer(
            "shadow",
            (4.0, 4.0),
            (-16.0, -16.0, 32.0, 32.0),
            LayerRot::Static(45.0),
            Before,
        ),
        layer(
            "monolith",
            (0.0, 0.0),
            (-17.0, -17.0, 34.0, 34.0),
            LayerRot::Static(45.0),
            Before,
        ),
        layer(
            "seams",
            (0.0, 0.0),
            (-22.0, -22.0, 44.0, 44.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "escort",
            (0.0, 0.0),
            (-35.0, -39.0, 70.0, 78.0),
            LayerRot::None,
            After,
        ),
    ],
];
