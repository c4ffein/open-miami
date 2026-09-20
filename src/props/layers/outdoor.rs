//! The OUTDOOR family: layer tables of props 24..41 (index = prop id - 24).

use super::*;

pub(super) const LAYERS: [&[LayerDef]; 18] = [
    // ======================= OUTDOOR: gate / parking lot =======================
    // 24 CAR / POD
    &[
        layer(
            "body",
            (0.0, 0.0),
            (-21.0, -30.0, 46.0, 64.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "glass",
            (0.0, 0.0),
            (-15.0, -22.0, 30.0, 44.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "lidar",
            (0.0, -2.0),
            (-6.0, -6.0, 12.0, 12.0),
            LayerRot::Spin { hz: 0.8 },
            After,
        ),
        layer(
            "lights",
            (0.0, 0.0),
            (-21.0, -31.0, 42.0, 66.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 25 CAR / SEDAN
    &[
        layer(
            "body",
            (0.0, 0.0),
            (-23.0, -44.0, 50.0, 84.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "glass",
            (0.0, 0.0),
            (-19.0, -30.0, 38.0, 58.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "lidar",
            (0.0, -8.0),
            (-6.0, -6.0, 12.0, 12.0),
            LayerRot::Spin { hz: 1.2 },
            After,
        ),
        layer(
            "lights",
            (0.0, 0.0),
            (-31.0, -45.0, 62.0, 100.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 26 CAR / OPEN
    &[
        layer(
            "spill",
            (0.0, 0.0),
            (-48.0, -26.0, 96.0, 52.0),
            LayerRot::None,
            After,
        ),
        layer(
            "body",
            (0.0, 0.0),
            (-23.0, -44.0, 50.0, 84.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "door l",
            (-21.0, -6.0),
            (-3.0, -1.0, 6.0, 26.0),
            LayerRot::Static(70.0),
            Before,
        ),
        layer(
            "door r",
            (21.0, -6.0),
            (-3.0, -1.0, 6.0, 26.0),
            LayerRot::Static(-70.0),
            Before,
        ),
        layer(
            "cabin",
            (0.0, 0.0),
            (-19.0, -30.0, 38.0, 58.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "lidar",
            (0.0, -8.0),
            (-6.0, -6.0, 12.0, 12.0),
            LayerRot::Spin { hz: 0.25 },
            After,
        ),
        layer(
            "lights",
            (0.0, 0.0),
            (-23.0, -45.0, 46.0, 84.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 27 DELIVERY VAN
    &[
        layer(
            "body",
            (0.0, 0.0),
            (-25.0, -46.0, 54.0, 96.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "glass",
            (0.0, 0.0),
            (-25.0, 20.0, 50.0, 22.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "puck",
            (0.0, 26.0),
            (-5.0, -5.0, 10.0, 10.0),
            LayerRot::Spin { hz: 0.6 },
            After,
        ),
        layer(
            "lights",
            (0.0, 0.0),
            (-31.0, -47.0, 62.0, 100.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 28 CHARGE PAD
    &[
        layer(
            "pad",
            (0.0, 0.0),
            (-31.0, -46.0, 62.0, 80.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "rings",
            (0.0, 0.0),
            (-28.0, -28.0, 56.0, 56.0),
            LayerRot::None,
            After,
        ),
    ],
    // 29 CAR / CHARGING
    &[
        layer(
            "pad",
            (0.0, 0.0),
            (-31.0, -46.0, 62.0, 80.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "rings",
            (0.0, 0.0),
            (-32.0, -38.0, 64.0, 64.0),
            LayerRot::None,
            After,
        ),
        layer(
            "body",
            (0.0, 0.0),
            (-21.0, -36.0, 46.0, 64.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "glass",
            (0.0, 0.0),
            (-15.0, -28.0, 30.0, 44.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "lidar",
            (0.0, -8.0),
            (-6.0, -6.0, 12.0, 12.0),
            LayerRot::Spin { hz: 0.15 },
            After,
        ),
        layer(
            "charge",
            (0.0, 0.0),
            (-24.0, -38.0, 48.0, 62.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 30 MAIN GATE
    &[
        layer(
            "lane",
            (0.0, 0.0),
            (-34.0, -50.0, 68.0, 100.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "posts",
            (0.0, 0.0),
            (-48.0, 4.0, 96.0, 20.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "scan",
            (0.0, 0.0),
            (-46.0, 6.0, 92.0, 12.0),
            LayerRot::None,
            After,
        ),
        layer(
            "arm shadow",
            (-32.0, 16.0),
            (-1.0, -3.0, 64.0, 6.0),
            LayerRot::Anim(gate_angle),
            After,
        ),
        layer(
            "arm",
            (-36.0, 12.0),
            (-6.0, -6.0, 70.0, 12.0),
            LayerRot::Anim(gate_angle),
            After,
        ),
    ],
    // 31 GUARD BOOTH
    &[
        layer(
            "wash",
            (0.0, 0.0),
            (20.0, -30.0, 30.0, 60.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "booth",
            (0.0, 0.0),
            (-30.0, -30.0, 58.0, 66.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "ac fan",
            (-14.0, -12.0),
            (-7.0, -7.0, 14.0, 14.0),
            LayerRot::Spin { hz: 2.0 },
            After,
        ),
        layer(
            "beacon",
            (12.0, -18.0),
            (-14.0, -14.0, 28.0, 28.0),
            LayerRot::Spin { hz: 0.7 },
            After,
        ),
    ],
    // 32 BOLLARDS
    &[
        layer(
            "posts",
            (0.0, 0.0),
            (-42.0, -10.0, 90.0, 26.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "leds",
            (0.0, 0.0),
            (-36.0, -4.0, 72.0, 8.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 33 PLANTER
    &[
        layer(
            "box",
            (0.0, 0.0),
            (-36.0, -20.0, 76.0, 46.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "shrub",
            (0.0, -2.0),
            (-34.0, -16.0, 68.0, 32.0),
            LayerRot::Sway { deg: 2.0, hz: 0.25 },
            After,
        ),
        layer(
            "lamp",
            (0.0, 0.0),
            (26.0, -18.0, 12.0, 12.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 34 LAMP POST
    &[
        layer(
            "pool",
            (0.0, 0.0),
            (-36.0, -52.0, 92.0, 92.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "shadow",
            (0.0, 0.0),
            (-20.0, 8.0, 56.0, 44.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "mast",
            (0.0, 0.0),
            (-26.0, -14.0, 42.0, 42.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "head",
            (0.0, 0.0),
            (0.0, -14.0, 20.0, 16.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "moths",
            (10.0, -6.0),
            (-16.0, -16.0, 32.0, 32.0),
            LayerRot::None,
            After,
        ),
    ],
    // 35 EV BAY
    &[
        layer(
            "asphalt",
            (0.0, 0.0),
            (-45.0, -45.0, 90.0, 90.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "lines",
            (0.0, 0.0),
            (-40.0, -44.0, 80.0, 84.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "glyph",
            (0.0, 0.0),
            (-20.0, -30.0, 40.0, 56.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 36 CROSSWALK
    &[
        layer(
            "asphalt",
            (0.0, 0.0),
            (-45.0, -45.0, 90.0, 90.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "stripes",
            (0.0, 0.0),
            (-44.0, -30.0, 88.0, 60.0),
            LayerRot::None,
            Before,
        ),
    ],
    // 37 DRONE PAD
    &[
        layer(
            "pad",
            (0.0, 0.0),
            (-44.0, -44.0, 88.0, 88.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "beacons",
            (0.0, 0.0),
            (-34.0, -34.0, 68.0, 68.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "drone",
            (0.0, 0.0),
            (-22.0, -22.0, 48.0, 48.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "rotor a",
            (-16.0, -16.0),
            (-11.0, -2.0, 22.0, 4.0),
            LayerRot::Anim(|t| rotor_twitch(t, 0.0)),
            After,
        ),
        layer(
            "rotor b",
            (16.0, -16.0),
            (-11.0, -2.0, 22.0, 4.0),
            LayerRot::Anim(|t| rotor_twitch(t, 1.7)),
            After,
        ),
        layer(
            "rotor c",
            (-16.0, 16.0),
            (-11.0, -2.0, 22.0, 4.0),
            LayerRot::Anim(|t| rotor_twitch(t, 3.9)),
            After,
        ),
        layer(
            "rotor d",
            (16.0, 16.0),
            (-11.0, -2.0, 22.0, 4.0),
            LayerRot::Anim(|t| rotor_twitch(t, 5.1)),
            After,
        ),
    ],
    // 38 SCOOTER RACK
    &[
        layer(
            "rail",
            (0.0, 0.0),
            (-44.0, -36.0, 92.0, 16.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "scooter a",
            (-28.0, 0.0),
            (-9.0, -31.0, 22.0, 66.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "scooter b",
            (0.0, 0.0),
            (-9.0, -31.0, 22.0, 66.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "scooter c",
            (28.0, 2.0),
            (-9.0, -31.0, 22.0, 66.0),
            LayerRot::Static(9.0),
            Before,
        ),
    ],
    // 39 DRAIN GRATE
    &[
        layer(
            "puddle",
            (0.0, 0.0),
            (-30.0, -28.0, 66.0, 66.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "grate",
            (0.0, 0.0),
            (-24.0, -14.0, 48.0, 28.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "sheen",
            (0.0, 0.0),
            (-30.0, -28.0, 66.0, 66.0),
            LayerRot::None,
            After,
        ),
    ],
    // 40 HOLO BILLBOARD
    &[
        layer(
            "wash",
            (0.0, 0.0),
            (-50.0, -26.0, 100.0, 76.0),
            LayerRot::None,
            After,
        ),
        layer(
            "shadow",
            (0.0, 0.0),
            (-32.0, -32.0, 96.0, 24.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "slab",
            (0.0, 0.0),
            (-42.0, -46.0, 84.0, 24.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "glyphs",
            (0.0, 0.0),
            (-42.0, -33.0, 84.0, 9.0),
            LayerRot::None,
            After,
        ),
    ],
    // 41 DUMPSTER
    &[
        layer(
            "body",
            (0.0, 0.0),
            (-30.0, -22.0, 66.0, 50.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "lid l",
            (0.0, 0.0),
            (-30.0, -22.0, 30.0, 44.0),
            LayerRot::None,
            Before,
        ),
        layer(
            "lid r",
            (0.0, -22.0),
            (0.0, -9.0, 30.0, 9.0),
            LayerRot::Static(-4.0),
            Before,
        ),
        layer(
            "flies",
            (0.0, 0.0),
            (0.0, -18.0, 32.0, 32.0),
            LayerRot::None,
            After,
        ),
    ],
];
