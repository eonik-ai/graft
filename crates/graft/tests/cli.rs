// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;
use std::process::Command;

fn graft() -> Command {
    Command::new(env!("CARGO_BIN_EXE_graft"))
}

fn example_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hook-v3-body-v1-9x16")
}

#[test]
fn signal_hook_rate_matches_worked_example() {
    let output = graft()
        .args([
            "-C",
            example_dir().to_str().unwrap(),
            "signal",
            "--kind",
            "hook_rate",
            "--t",
            "0-3",
        ])
        .output()
        .expect("run graft signal");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let got: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let expected: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(example_dir().join("dirty.json")).unwrap())
            .unwrap();
    assert_eq!(got["slots"], expected["slots"]);
    assert_eq!(got["kerfs"], expected["kerfs"]);
    assert_eq!(got["clean"], expected["clean"]);
    assert_eq!(got["warnings"], expected["warnings"]);
    assert_eq!(got["signal"]["kind"], "hook_rate");
    assert!(!got["slots"].as_array().unwrap().iter().any(|s| s == "body"));
}

#[test]
fn compile_is_a_plan_not_an_encoder() {
    let output = graft()
        .args(["-C", example_dir().to_str().unwrap(), "compile"])
        .output()
        .expect("run graft compile");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let got: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(got["encode"], false);
    assert_eq!(got["dest_id"], "9x16");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("plan only"));
}

#[test]
fn init_slot_scion_bind_signal_roundtrip() {
    let tmp = std::env::temp_dir().join(format!("graft-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let dir = tmp.to_str().unwrap();

    assert!(graft()
        .args(["-C", dir, "init"])
        .status()
        .unwrap()
        .success());
    assert!(graft()
        .args(["-C", dir, "slot", "body", "--span", "3-20"])
        .status()
        .unwrap()
        .success());
    assert!(graft()
        .args(["-C", dir, "scion", "9x16", "--dest", "1080x1920"])
        .status()
        .unwrap()
        .success());
    let hook_mat = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let body_mat = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    assert!(graft()
        .args(["-C", dir, "bind", "hook", hook_mat, "--in", "0.4", "--out", "3.4"])
        .status()
        .unwrap()
        .success());
    assert!(graft()
        .args(["-C", dir, "bind", "body", body_mat, "--in", "1.2", "--out", "18.2"])
        .status()
        .unwrap()
        .success());

    let output = graft()
        .args(["-C", dir, "signal", "--kind", "hook_rate", "--t", "0-3"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let got: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(got["slots"], serde_json::json!(["hook"]));
    assert_eq!(got["kerfs"], serde_json::json!([["hook", "body"]]));
    assert_eq!(got["clean"], serde_json::json!(["body"]));

    let _ = std::fs::remove_dir_all(&tmp);
}
