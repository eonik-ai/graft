// SPDX-License-Identifier: Apache-2.0

//! Lossy guest adapters. Depends on graft-score only.
//!
//! One flattened timeline lowers to OTIO, FCPXML, FCP7 XMEML, and MLT.

mod guest;

use std::collections::BTreeMap;
use std::path::Path;

use graft_score::{FrameRange, Join, Marker, Result, Scion, Score};

pub use guest::{
    apply_guest_edits, export_bundle, export_fcpxml, export_mlt, export_xmeml, finish_import,
    is_capcut_draft, parse_guest_dir, parse_guest_file, write_otio, ParsedClip, ParsedGuest,
    FINAL_CUT, LOSS_NAME, PREMIERE, RESOLVE_OTIO, SHOTCUT,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestSlot {
    pub id: String,
    pub range: FrameRange,
}

#[derive(Debug)]
pub struct OtioImport {
    pub scion: Scion,
    pub loss: serde_json::Value,
    pub joins: Vec<Join>,
    pub markers: Vec<Marker>,
    pub slots: Vec<GuestSlot>,
}

pub fn export_otio(score: &Score, scion: &Scion, out: &Path) -> Result<serde_json::Value> {
    write_otio(score, scion, out, &BTreeMap::new())
}

pub fn import_otio(score: &Score, file: &Path, id: &str) -> Result<OtioImport> {
    let parsed = parse_guest_file(file)?;
    if parsed.format != "otio" {
        return Err(graft_score::Error::invalid(format!(
            "{} is not an OTIO Timeline.1 document",
            file.display()
        )));
    }
    finish_import(score, id, &parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_score::{effective_bindings, load_scion, load_score};

    #[test]
    fn example_roundtrip_preserves_bindings() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/hook-v3-body-v1-9x16");
        let score = load_score(&dir.join("score.json")).unwrap();
        let scion = load_scion(&dir.join("scion.json")).unwrap();
        let tmp = std::env::temp_dir().join(format!("graft-otio-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let out = tmp.join("timeline.otio.json");
        export_otio(&score, &scion, &out).unwrap();
        let imported = import_otio(&score, &out, "imported").unwrap();
        let loss = imported.loss;
        let imported = imported.scion;
        let before = effective_bindings(&score, &scion).unwrap();
        let after = effective_bindings(&score, &imported).unwrap();
        assert_eq!(before.len(), after.len());
        for (id, binding) in before {
            assert_eq!(after[&id].material, binding.material);
            assert_eq!(after[&id].source, binding.source);
        }
        assert_eq!(loss["lost"][0], "effects");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sidecar_restores_graph_and_overlay_track() {
        use graft_score::{CompositionGraph, GraphMode, GraphNode, Role};
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/hook-v3-body-v1-9x16");
        let mut score = load_score(&dir.join("score.json")).unwrap();
        score.slots.push(graft_score::Slot {
            id: "overlay".into(),
            role: Role::Overlay,
            range: graft_score::FrameRange::new(0, 90),
            optional: true,
            window: None,
        });
        let mut scion = load_scion(&dir.join("scion.json")).unwrap();
        scion.graph = Some(CompositionGraph {
            mode: GraphMode::Append,
            nodes: vec![GraphNode {
                id: "eq".into(),
                op: "eq".into(),
                inputs: vec!["#picture".into()],
                material: None,
                source: None,
                place: None,
                enable: Some("between(t,0,3)".into()),
                role: None,
                params: Some(serde_json::json!({"saturation": "1.1"})),
                keys: None,
                nodes: Vec::new(),
            }],
        });
        let overlay_mat = scion
            .layers
            .first()
            .and_then(|layer| layer.bindings.values().next())
            .map(|b| b.material.clone())
            .unwrap();
        let hook = scion.layers[0].bindings["hook"].clone();
        scion.layers[0].bindings.insert(
            "overlay".into(),
            graft_score::Binding {
                material: overlay_mat,
                source: hook.source,
                params: None,
                audio: None,
                nested: None,
                x: Some("8".into()),
                y: Some("16".into()),
                opacity: Some(0.8),
                motion: None,
                remap: Vec::new(),
            },
        );
        scion.validate_against_score(&score).unwrap();
        let tmp = std::env::temp_dir().join(format!("graft-otio-graph-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let out = tmp.join("timeline.otio.json");
        score.joins.push(graft_score::Join {
            left: "hook".into(),
            right: "body".into(),
            transition: "fadeblack".into(),
            duration_frames: 6,
        });
        score.markers.push(graft_score::Marker {
            id: "punch".into(),
            range: graft_score::FrameRange::new(0, 30),
        });
        export_otio(&score, &scion, &out).unwrap();
        let imported = import_otio(&score, &out, "imported").unwrap();
        assert_eq!(imported.scion.graph, scion.graph);
        assert!(imported.scion.layers[0].bindings.contains_key("overlay"));
        assert_eq!(
            imported.scion.layers[0].bindings["overlay"].x.as_deref(),
            Some("8")
        );
        assert_eq!(
            imported.scion.layers[0].bindings["overlay"].y.as_deref(),
            Some("16")
        );
        assert_eq!(
            imported.scion.layers[0].bindings["overlay"].opacity,
            Some(0.8)
        );
        assert_eq!(imported.joins[0].transition, "fadeblack");
        assert_eq!(imported.markers[0].id, "punch");
        assert!(imported.loss["preserved"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_str() == Some("scion.graph (graft sidecar)")));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
