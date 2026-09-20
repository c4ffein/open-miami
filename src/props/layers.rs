//! `PROP_LAYERS`: the layers of every prop, one table per FAMILY (the family
//! files hold the data; ids are contiguous, see `PROP_FAMILIES`).

use super::*;

mod datacenter;
mod lobby;
mod outdoor;

/// The layers of every prop, bottom to top (index = prop id, see
/// [`PROP_NAMES`]). The drawing itself is `draw_prop_layer` (`props/draw.rs`).
pub static PROP_LAYERS: [&[LayerDef]; PROP_COUNT] = join();

/// The three family tables back to back, in id order.
const fn join() -> [&'static [LayerDef]; PROP_COUNT] {
    let mut out: [&[LayerDef]; PROP_COUNT] = [&[]; PROP_COUNT];
    let mut n = 0;
    let mut i = 0;
    while i < datacenter::LAYERS.len() {
        out[n] = datacenter::LAYERS[i];
        n += 1;
        i += 1;
    }
    i = 0;
    while i < outdoor::LAYERS.len() {
        out[n] = outdoor::LAYERS[i];
        n += 1;
        i += 1;
    }
    i = 0;
    while i < lobby::LAYERS.len() {
        out[n] = lobby::LAYERS[i];
        n += 1;
        i += 1;
    }
    assert!(n == PROP_COUNT, "the family tables must cover every prop");
    out
}
