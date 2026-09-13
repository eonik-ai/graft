// SPDX-License-Identifier: Apache-2.0

//! USD-style: layers are opinions with strength. Flatten to one binding
//! per slot **before** the action graph. Not a pixel blend.

use std::collections::BTreeMap;

use graft_score::{effective_bindings, Binding, Layer, Scion, Score};

/// One binding per slot after layer strength is resolved.
#[derive(Clone, Debug)]
pub struct Flattened {
    pub bindings: BTreeMap<String, Binding>,
}

#[allow(dead_code)]
pub fn layer_strength(layer: Layer) -> u8 {
    match layer {
        Layer::Base => 0,
        Layer::Copy => 1,
        Layer::Grade => 2,
        Layer::Legal => 3,
    }
}

/// Strongest declared layer wins per slot. Inheritance is resolved before
/// this flatten so the scion already carries parent opinions.
pub fn flatten(score: &Score, scion: &Scion) -> Result<Flattened, graft_score::Error> {
    Ok(Flattened {
        bindings: effective_bindings(score, scion)?,
    })
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
