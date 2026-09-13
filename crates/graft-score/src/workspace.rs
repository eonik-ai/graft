// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{Binding, BindingLayer, Error, Layer, Result, Scion, Score};

pub fn resolve_scion(scions: &BTreeMap<String, Scion>, id: &str) -> Result<Scion> {
    fn visit(
        scions: &BTreeMap<String, Scion>,
        id: &str,
        visiting: &mut BTreeSet<String>,
    ) -> Result<Scion> {
        if !visiting.insert(id.to_string()) {
            return Err(Error::invalid(format!("scion parent cycle at {id}")));
        }
        let child = scions
            .get(id)
            .ok_or_else(|| Error::invalid(format!("unknown scion {id}")))?;
        let mut resolved = if let Some(parent) = &child.parent {
            let mut parent = visit(scions, parent, visiting)?;
            if parent.concept != child.concept {
                return Err(Error::invalid(format!(
                    "scion {} concept differs from parent",
                    child.id
                )));
            }
            parent.id = child.id.clone();
            parent.dest = child.dest.clone();
            parent
        } else {
            child.clone()
        };
        for child_layer in &child.layers {
            let target = resolved.ensure_layer(child_layer.name);
            target.bindings.extend(child_layer.bindings.clone());
        }
        resolved.id = child.id.clone();
        resolved.parent = None;
        visiting.remove(id);
        Ok(resolved)
    }

    visit(scions, id, &mut BTreeSet::new())
}

pub fn effective_bindings(score: &Score, scion: &Scion) -> Result<BTreeMap<String, Binding>> {
    scion.validate_against_score(score)?;
    let mut layers: Vec<&BindingLayer> = scion.layers.iter().collect();
    layers.sort_by_key(|layer| {
        score
            .layers
            .iter()
            .position(|name| *name == layer.name)
            .unwrap_or(usize::MAX)
    });
    let mut out = BTreeMap::new();
    for layer in layers {
        out.extend(layer.bindings.clone());
    }
    for slot in &score.slots {
        if !slot.optional && !out.contains_key(&slot.id) {
            return Err(Error::invalid(format!(
                "required slot {} is unbound after inheritance/layers",
                slot.id
            )));
        }
    }
    Ok(out)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SemanticChange {
    pub path: String,
    pub before: serde_json::Value,
    pub after: serde_json::Value,
    pub cache: String,
}

pub fn semantic_diff(score: &Score, left: &Scion, right: &Scion) -> Result<Vec<SemanticChange>> {
    let left_bindings = effective_bindings(score, left)?;
    let right_bindings = effective_bindings(score, right)?;
    let mut changes = Vec::new();
    if left.dest != right.dest {
        changes.push(SemanticChange {
            path: "dest".into(),
            before: serde_json::to_value(&left.dest).map_err(|e| Error::invalid(e.to_string()))?,
            after: serde_json::to_value(&right.dest).map_err(|e| Error::invalid(e.to_string()))?,
            cache: "all slot encodes and joins miss".into(),
        });
    }
    let ids: BTreeSet<String> = left_bindings
        .keys()
        .chain(right_bindings.keys())
        .cloned()
        .collect();
    for id in ids {
        let before = left_bindings.get(&id);
        let after = right_bindings.get(&id);
        if before != after {
            changes.push(SemanticChange {
                path: format!("slots.{id}.binding"),
                before: serde_json::to_value(before).map_err(|e| Error::invalid(e.to_string()))?,
                after: serde_json::to_value(after).map_err(|e| Error::invalid(e.to_string()))?,
                cache: format!("{id} encode and adjacent joins miss"),
            });
        }
    }
    Ok(changes)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MergeConflict {
    pub path: String,
    pub base: serde_json::Value,
    pub ours: serde_json::Value,
    pub theirs: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MergeOutcome {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merged: Option<Scion>,
    pub conflicts: Vec<MergeConflict>,
}

pub fn merge_scions(
    score: &Score,
    id: &str,
    base: &Scion,
    ours: &Scion,
    theirs: &Scion,
) -> Result<MergeOutcome> {
    let b = effective_bindings(score, base)?;
    let o = effective_bindings(score, ours)?;
    let t = effective_bindings(score, theirs)?;
    let mut conflicts = Vec::new();
    let dest = merge_value("dest", &base.dest, &ours.dest, &theirs.dest, &mut conflicts)?;
    let ids: BTreeSet<String> = b.keys().chain(o.keys()).chain(t.keys()).cloned().collect();
    let mut bindings = BTreeMap::new();
    for slot in ids {
        if let Some(value) = merge_optional(
            &format!("slots.{slot}.binding"),
            b.get(&slot),
            o.get(&slot),
            t.get(&slot),
            &mut conflicts,
        )? {
            bindings.insert(slot, value);
        }
    }
    if !conflicts.is_empty() {
        return Ok(MergeOutcome {
            merged: None,
            conflicts,
        });
    }
    Ok(MergeOutcome {
        merged: Some(Scion {
            graft: crate::GRAFT_SCHEMA.into(),
            id: id.into(),
            concept: base.concept.clone(),
            parent: None,
            change_request: None,
            dest,
            layers: vec![BindingLayer {
                name: Layer::Base,
                bindings,
            }],
        }),
        conflicts,
    })
}

fn merge_value<T>(
    path: &str,
    base: &T,
    ours: &T,
    theirs: &T,
    conflicts: &mut Vec<MergeConflict>,
) -> Result<T>
where
    T: Clone + PartialEq + Serialize,
{
    if ours == theirs {
        return Ok(ours.clone());
    }
    if ours == base {
        return Ok(theirs.clone());
    }
    if theirs == base {
        return Ok(ours.clone());
    }
    conflicts.push(MergeConflict {
        path: path.into(),
        base: serde_json::to_value(base).map_err(|e| Error::invalid(e.to_string()))?,
        ours: serde_json::to_value(ours).map_err(|e| Error::invalid(e.to_string()))?,
        theirs: serde_json::to_value(theirs).map_err(|e| Error::invalid(e.to_string()))?,
    });
    Ok(ours.clone())
}

fn merge_optional<T>(
    path: &str,
    base: Option<&T>,
    ours: Option<&T>,
    theirs: Option<&T>,
    conflicts: &mut Vec<MergeConflict>,
) -> Result<Option<T>>
where
    T: Clone + PartialEq + Serialize,
{
    if ours == theirs {
        return Ok(ours.cloned());
    }
    if ours == base {
        return Ok(theirs.cloned());
    }
    if theirs == base {
        return Ok(ours.cloned());
    }
    conflicts.push(MergeConflict {
        path: path.into(),
        base: serde_json::to_value(base).map_err(|e| Error::invalid(e.to_string()))?,
        ours: serde_json::to_value(ours).map_err(|e| Error::invalid(e.to_string()))?,
        theirs: serde_json::to_value(theirs).map_err(|e| Error::invalid(e.to_string()))?,
    });
    Ok(ours.cloned())
}

pub fn validate_family(score: &Score, scions: &BTreeMap<String, Scion>) -> Result<()> {
    for scion in scions.values() {
        scion.validate_against_score(score)?;
        let resolved = resolve_scion(scions, &scion.id)?;
        let _ = effective_bindings(score, &resolved);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Binding, Dest, Encoder, FrameRange, FrameRate, Role, Slot, TimedRange, Window, GRAFT_SCHEMA,
    };
    use crate::{Clock, Score};

    fn score() -> Score {
        Score {
            graft: GRAFT_SCHEMA.into(),
            concept: "11111111-1111-4111-8111-111111111111".into(),
            clock: Clock {
                rate: FrameRate::new(30, 1),
                duration_frames: 90,
            },
            slots: vec![Slot {
                id: "hook".into(),
                role: Role::Hook,
                range: FrameRange::new(0, 90),
                optional: false,
                window: Some(Window {
                    kind: "hook_rate".into(),
                    range: FrameRange::new(0, 90),
                }),
            }],
            layers: vec![Layer::Base, Layer::Legal],
            dest_default: Some("9x16".into()),
            spill_threshold_frames: 11,
        }
    }

    fn dest() -> Dest {
        Dest {
            id: "9x16".into(),
            width: 1080,
            height: 1920,
            rate: FrameRate::new(30, 1),
            pix_fmt: "yuv420p".into(),
            color: "bt709".into(),
            encoder: Encoder::graft_intra(),
        }
    }

    fn bind(tag: u8) -> Binding {
        Binding {
            material: format!("blake3:{:064x}", tag),
            source: TimedRange {
                rate: FrameRate::new(30, 1),
                range: FrameRange::new(0, 90),
            },
            params: None,
            audio: None,
        }
    }

    fn scion(id: &str, parent: Option<&str>, layer: Layer, binding: Binding) -> Scion {
        Scion {
            graft: GRAFT_SCHEMA.into(),
            id: id.into(),
            concept: "11111111-1111-4111-8111-111111111111".into(),
            parent: parent.map(str::to_string),
            change_request: None,
            dest: dest(),
            layers: vec![BindingLayer {
                name: layer,
                bindings: BTreeMap::from([("hook".into(), binding)]),
            }],
        }
    }

    #[test]
    fn child_inherits_parent_and_legal_overrides_base() {
        let score = score();
        let parent = scion("root", None, Layer::Base, bind(1));
        let child = scion("child", Some("root"), Layer::Legal, bind(2));
        let family = BTreeMap::from([(parent.id.clone(), parent), (child.id.clone(), child)]);
        let resolved = resolve_scion(&family, "child").unwrap();
        let bindings = effective_bindings(&score, &resolved).unwrap();
        assert_eq!(
            bindings["hook"].material,
            "blake3:0000000000000000000000000000000000000000000000000000000000000002"
        );
    }

    #[test]
    fn cycle_is_rejected() {
        let a = scion("a", Some("b"), Layer::Base, bind(1));
        let mut b = scion("b", Some("a"), Layer::Base, bind(1));
        b.parent = Some("a".into());
        let family = BTreeMap::from([(a.id.clone(), a), (b.id.clone(), b)]);
        assert!(resolve_scion(&family, "a").is_err());
    }

    #[test]
    fn merge_keeps_disjoint_slots() {
        let score = score();
        let base = scion("base", None, Layer::Base, bind(1));
        let ours = scion("ours", None, Layer::Base, bind(1));
        let theirs = scion("theirs", None, Layer::Base, bind(2));
        let expected = theirs.layers[0].bindings["hook"].material.clone();
        let outcome = merge_scions(&score, "merged", &base, &ours, &theirs).unwrap();
        assert!(outcome.conflicts.is_empty());
        let merged = outcome.merged.unwrap();
        assert_eq!(
            effective_bindings(&score, &merged).unwrap()["hook"].material,
            expected
        );
    }
}
