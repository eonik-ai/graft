// SPDX-License-Identifier: Apache-2.0

//! USD-style: layers are opinions with strength. Flatten to one binding
//! per slot **before** the action graph. Not a pixel blend.

use std::collections::BTreeMap;

use graft_score::{Binding, Layer, Scion, Score};

/// One binding per slot after layer strength is resolved.
#[derive(Clone, Debug)]
pub struct Flattened {
    pub bindings: BTreeMap<String, Binding>,
}

pub fn layer_strength(layer: Layer) -> u8 {
    match layer {
        Layer::Base => 0,
        Layer::Copy => 1,
        Layer::Grade => 2,
        Layer::Legal => 3,
    }
}

/// Schema 0.1.0 has a single binding map on the scion. Strength is recorded
/// so overlay layers can win later; today flatten is that map.
pub fn flatten(score: &Score, scion: &Scion) -> Flattened {
    let _ = strongest_layer(score);
    Flattened {
        bindings: scion.bindings.clone(),
    }
}

fn strongest_layer(score: &Score) -> Layer {
    score
        .layers
        .iter()
        .copied()
        .max_by_key(|l| layer_strength(*l))
        .unwrap_or(Layer::Base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_outweighs_base() {
        assert!(layer_strength(Layer::Legal) > layer_strength(Layer::Base));
        assert!(layer_strength(Layer::Grade) > layer_strength(Layer::Copy));
    }
}
