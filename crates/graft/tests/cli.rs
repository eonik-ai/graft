// SPDX-License-Identifier: Apache-2.0

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn graft() -> Command {
    Command::new(env!("CARGO_BIN_EXE_graft"))
}

fn example_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hook-v3-body-v1-9x16")
}

fn tmp(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "graft-cli-{name}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn ffmpeg_available() -> bool {
    Command::new(std::env::var("FFMPEG").unwrap_or_else(|_| "ffmpeg".into()))
        .args(["-hide_banner", "-version"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn write_color(dir: &Path, name: &str, color: &str, seconds: &str, rate: &str) -> PathBuf {
    let ffmpeg = std::env::var("FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
    let path = dir.join(name);
    let status = Command::new(&ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={color}:s=64x64:d={seconds}:r={rate}"),
            "-f",
            "lavfi",
            "-i",
            &format!("sine=frequency=440:duration={seconds}"),
            "-shortest",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            path.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "ffmpeg lavfi {name}");
    path
}

fn run_ok(dir: &Path, args: &[&str]) -> serde_json::Value {
    let output = graft()
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("run graft");
    assert!(
        output.status.success(),
        "graft {} failed\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if output.stdout.is_empty() {
        return serde_json::json!(null);
    }
    serde_json::from_slice(&output.stdout).unwrap_or(serde_json::json!(null))
}

fn seed_example_build(dir: &Path) -> String {
    let id = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    let root = dir.join(".graft/builds").join(id);
    fs::create_dir_all(&root).unwrap();
    fs::copy(dir.join("build.json"), root.join("build.json")).unwrap();
    id.to_string()
}

fn copy_example() -> PathBuf {
    let dir = tmp("example");
    for name in [
        "score.json",
        "scion.json",
        "time-map.json",
        "dirty.json",
        "build.json",
    ] {
        fs::copy(example_dir().join(name), dir.join(name)).unwrap();
    }
    fs::create_dir_all(dir.join("scions")).unwrap();
    fs::copy(
        example_dir().join("scions/hook_v3+body_v1+cta_v1@9x16.json"),
        dir.join("scions/hook_v3+body_v1+cta_v1@9x16.json"),
    )
    .unwrap();
    dir
}

#[test]
fn signal_hook_rate_matches_worked_example() {
    let dir = copy_example();
    let build = seed_example_build(&dir);
    let got = run_ok(&dir, &["signal", "--kind", "hook_rate", "--build", &build]);
    let expected: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("dirty.json")).unwrap()).unwrap();
    assert_eq!(got["slots"], expected["slots"]);
    assert_eq!(got["kerfs"], expected["kerfs"]);
    assert_eq!(got["clean"], expected["clean"]);
    assert_eq!(got["warnings"], expected["warnings"]);
    assert_eq!(got["signal"]["kind"], "hook_rate");
    assert_eq!(
        got["signal"]["range"],
        serde_json::json!({"start": 0, "duration": 90})
    );
    assert!(!got["slots"].as_array().unwrap().iter().any(|s| s == "body"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn compile_is_a_plan_not_an_encoder() {
    let dir = copy_example();
    let got = run_ok(&dir, &["compile"]);
    assert_eq!(got["plan"]["encode"], false);
    assert_eq!(got["plan"]["dest_id"], "9x16");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn workspace_fork_diff_merge_feedback_and_otio() {
    let dir = tmp("workspace");
    run_ok(&dir, &["init"]);
    run_ok(&dir, &["slot", "body", "--span", "3-20"]);
    run_ok(
        &dir,
        &[
            "scion",
            "create",
            "root",
            "--dest",
            "1080x1920",
            "--encoder",
            "graft-intra",
        ],
    );
    let hook = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let body = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    run_ok(
        &dir,
        &[
            "bind", "hook", hook, "--in", "0", "--out", "3", "--scion", "root",
        ],
    );
    run_ok(
        &dir,
        &[
            "bind", "body", body, "--in", "0", "--out", "17", "--scion", "root",
        ],
    );
    run_ok(&dir, &["scion", "fork", "root", "hook-b"]);
    run_ok(
        &dir,
        &[
            "bind",
            "hook",
            "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "--in",
            "0",
            "--out",
            "3",
            "--scion",
            "hook-b",
        ],
    );
    let diff = run_ok(&dir, &["diff", "root", "hook-b"]);
    assert!(diff
        .as_array()
        .unwrap()
        .iter()
        .any(|change| { change["path"] == "slots.hook.binding" }));
    run_ok(&dir, &["scion", "fork", "root", "lang"]);
    run_ok(&dir, &["merge", "root", "hook-b", "lang", "--id", "merged"]);
    let compile = run_ok(&dir, &["compile", "--scion", "root"]);
    let build = compile["build"]["id"].as_str().unwrap().to_string();
    let feedback = dir.join("hook-rate.json");
    fs::write(
        &feedback,
        serde_json::json!({
            "id": "fb1",
            "build": build,
            "kind": "hook_rate"
        })
        .to_string(),
    )
    .unwrap();
    run_ok(&dir, &["feedback", "ingest", "hook-rate.json"]);
    let iterate = run_ok(
        &dir,
        &[
            "iterate",
            "--from",
            &build,
            "--feedback",
            "fb1",
            "--scion",
            "iter",
        ],
    );
    assert_eq!(iterate["dirty_slots"], serde_json::json!(["hook"]));
    assert_eq!(iterate["creative_replacement"], "required");
    let export = dir.join("timeline.otio.json");
    run_ok(
        &dir,
        &[
            "export",
            "otio",
            "--scion",
            "root",
            "--out",
            export.to_str().unwrap(),
        ],
    );
    assert!(export.is_file());
    run_ok(
        &dir,
        &[
            "import",
            "otio",
            export.to_str().unwrap(),
            "--scion",
            "from-otio",
        ],
    );
    let status = run_ok(&dir, &["status"]);
    assert!(status["recipe_files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f == "score.json"));
    let remote = dir.join("object-store");
    run_ok(
        &dir,
        &["store", "push", "--remote", remote.to_str().unwrap()],
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn migrate_rewrites_0_1_0_fixtures() {
    let dir = tmp("migrate");
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/migration-0.1.0");
    fs::copy(fixtures.join("score.json"), dir.join("score.json")).unwrap();
    fs::copy(fixtures.join("scion.json"), dir.join("scion.json")).unwrap();
    run_ok(&dir, &["migrate"]);
    let score: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("score.json")).unwrap()).unwrap();
    assert_eq!(score["graft"], "0.2.0");
    assert_eq!(score["clock"]["rate"]["num"], 30);
    assert_eq!(score["slots"][0]["range"]["duration"], 90);
    let scion: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(dir.join("scions/hook_v3+body_v1+cta_v1@9x16.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        scion["layers"][0]["bindings"]["hook"]["source"]["range"]["start"],
        12
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn persisted_hook_swap_reuses_body_payload() {
    if !ffmpeg_available() {
        eprintln!("skip persisted_hook_swap_reuses_body_payload — install ffmpeg");
        return;
    }
    let dir = tmp("reuse");
    run_ok(&dir, &["init"]);
    run_ok(&dir, &["slot", "hook", "--span", "0-1"]);
    run_ok(&dir, &["slot", "body", "--span", "1-3"]);
    run_ok(
        &dir,
        &[
            "scion",
            "create",
            "ad",
            "--dest",
            "64x64",
            "--encoder",
            "x264",
        ],
    );
    let hook_a = write_color(&dir, "hook-a.mp4", "red", "1.2", "10");
    let hook_b = write_color(&dir, "hook-b.mp4", "blue", "1.2", "10");
    let body = write_color(&dir, "body.mp4", "green", "2.2", "10");
    run_ok(
        &dir,
        &[
            "bind",
            "hook",
            hook_a.to_str().unwrap(),
            "--in",
            "0",
            "--out",
            "1",
        ],
    );
    run_ok(
        &dir,
        &[
            "bind",
            "body",
            body.to_str().unwrap(),
            "--in",
            "0",
            "--out",
            "2",
        ],
    );
    let first = run_ok(&dir, &["compile", "--out", "one.mp4"]);
    assert_eq!(first["plan"]["encode"], true);
    let warmed = run_ok(&dir, &["compile", "--out", "one-warm.mp4"]);
    let body_blob = warmed["plan"]["slots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|slot| slot["id"] == "body")
        .unwrap()["blob"]
        .clone();
    run_ok(
        &dir,
        &[
            "bind",
            "hook",
            hook_b.to_str().unwrap(),
            "--in",
            "0",
            "--out",
            "1",
        ],
    );
    let dirty = run_ok(&dir, &["dirty"]);
    assert_eq!(dirty["clean"], serde_json::json!(["body"]));
    let second = run_ok(&dir, &["compile", "--out", "two.mp4"]);
    let body2 = second["plan"]["slots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|slot| slot["id"] == "body")
        .unwrap();
    assert_eq!(body2["cache"], "hit");
    assert_eq!(body2["blob"], body_blob);
    let _ = fs::remove_dir_all(&dir);
}

/// Mission story on generated media: init → scions → compile → signal →
/// iterate → reuse clean encodes → OTIO guest → preview → store push.
#[test]
fn founding_loop_on_generated_media() {
    if !ffmpeg_available() {
        eprintln!("skip founding_loop_on_generated_media — install ffmpeg");
        return;
    }
    let dir = tmp("founding");
    run_ok(&dir, &["init"]);
    run_ok(
        &dir,
        &[
            "slot",
            "hook",
            "--span",
            "0-1",
            "--window",
            "0-1",
            "--window-kind",
            "hook_rate",
        ],
    );
    run_ok(&dir, &["slot", "body", "--span", "1-3"]);
    run_ok(&dir, &["slot", "cta", "--span", "3-4", "--role", "cta"]);
    run_ok(
        &dir,
        &[
            "scion",
            "create",
            "9x16",
            "--dest",
            "64x64",
            "--encoder",
            "x264",
        ],
    );
    let hook_a = write_color(&dir, "hook-a.mp4", "red", "1.2", "30");
    let hook_b = write_color(&dir, "hook-b.mp4", "blue", "1.2", "30");
    let body = write_color(&dir, "body.mp4", "green", "2.2", "30");
    let cta = write_color(&dir, "cta.mp4", "yellow", "1.2", "30");
    run_ok(
        &dir,
        &[
            "bind",
            "hook",
            hook_a.to_str().unwrap(),
            "--in",
            "0",
            "--out",
            "1",
            "--scion",
            "9x16",
        ],
    );
    run_ok(
        &dir,
        &[
            "bind",
            "body",
            body.to_str().unwrap(),
            "--in",
            "0",
            "--out",
            "2",
            "--scion",
            "9x16",
        ],
    );
    run_ok(
        &dir,
        &[
            "bind",
            "cta",
            cta.to_str().unwrap(),
            "--in",
            "0",
            "--out",
            "1",
            "--scion",
            "9x16",
        ],
    );

    run_ok(&dir, &["scion", "fork", "9x16", "hook-v2"]);
    let hook_child: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("scions/hook-v2.json")).unwrap())
            .unwrap();
    assert_eq!(hook_child["parent"], "9x16");
    assert!(hook_child["layers"]
        .as_array()
        .map(|layers| layers.is_empty())
        .unwrap_or(false));
    run_ok(
        &dir,
        &[
            "bind",
            "hook",
            hook_b.to_str().unwrap(),
            "--in",
            "0",
            "--out",
            "1",
            "--scion",
            "hook-v2",
        ],
    );
    let hook_bound: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("scions/hook-v2.json")).unwrap())
            .unwrap();
    let bindings = &hook_bound["layers"][0]["bindings"];
    assert!(bindings.get("hook").is_some());
    assert!(bindings.get("body").is_none());
    assert!(bindings.get("cta").is_none());

    run_ok(&dir, &["scion", "fork", "9x16", "lang"]);
    let lang: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("scions/lang.json")).unwrap()).unwrap();
    assert!(lang["layers"]
        .as_array()
        .map(|layers| layers.is_empty())
        .unwrap_or(false));

    run_ok(
        &dir,
        &[
            "scion",
            "create",
            "16x9",
            "--dest",
            "96x64",
            "--encoder",
            "x264",
        ],
    );
    let dest_variant: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("scions/16x9.json")).unwrap()).unwrap();
    assert_eq!(dest_variant["dest"]["width"], 96);
    assert_eq!(dest_variant["dest"]["height"], 64);
    assert!(dest_variant["layers"][0]["bindings"]
        .as_object()
        .map(|b| b.is_empty())
        .unwrap_or(false));

    let diff = run_ok(&dir, &["diff", "9x16", "hook-v2"]);
    assert!(diff
        .as_array()
        .unwrap()
        .iter()
        .any(|change| change["path"] == "slots.hook.binding"));
    assert!(!diff
        .as_array()
        .unwrap()
        .iter()
        .any(|change| change["path"] == "slots.body.binding"));
    let merge = run_ok(
        &dir,
        &["merge", "9x16", "hook-v2", "lang", "--id", "merged"],
    );
    assert!(merge["merged"].is_object());
    assert_eq!(merge["conflicts"], serde_json::json!([]));

    let first = run_ok(&dir, &["compile", "--scion", "9x16", "--out", "ad.mp4"]);
    assert_eq!(first["plan"]["encode"], true);
    assert!(dir.join("ad.mp4").is_file());
    let build = first["build"]["id"].as_str().unwrap().to_string();
    let warmed = run_ok(
        &dir,
        &["compile", "--scion", "9x16", "--out", "ad-warm.mp4"],
    );
    let slot_blob = |plan: &serde_json::Value, id: &str| {
        plan["plan"]["slots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|slot| slot["id"] == id)
            .unwrap()["blob"]
            .clone()
    };
    let body_blob = slot_blob(&warmed, "body");
    let cta_blob = slot_blob(&warmed, "cta");

    let signal = run_ok(&dir, &["signal", "--kind", "hook_rate", "--build", &build]);
    assert_eq!(signal["slots"], serde_json::json!(["hook"]));
    assert!(!signal["slots"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s == "body"));
    assert_eq!(signal["clean"], serde_json::json!(["body", "cta"]));

    let feedback = dir.join("hook-rate.json");
    fs::write(
        &feedback,
        serde_json::json!({
            "id": "fb-hook",
            "build": build,
            "kind": "hook_rate"
        })
        .to_string(),
    )
    .unwrap();
    run_ok(&dir, &["feedback", "ingest", "hook-rate.json"]);
    let iterate = run_ok(
        &dir,
        &[
            "iterate",
            "--from",
            &build,
            "--feedback",
            "fb-hook",
            "--scion",
            "hook-v3",
        ],
    );
    assert_eq!(iterate["dirty_slots"], serde_json::json!(["hook"]));
    assert_eq!(iterate["creative_replacement"], "required");
    assert_ne!(iterate["dirty_slots"], serde_json::json!(["hook", "body"]));
    run_ok(
        &dir,
        &[
            "bind",
            "hook",
            hook_b.to_str().unwrap(),
            "--in",
            "0",
            "--out",
            "1",
            "--scion",
            "hook-v3",
        ],
    );
    let second = run_ok(
        &dir,
        &["compile", "--scion", "hook-v3", "--out", "ad-v3.mp4"],
    );
    let body2 = second["plan"]["slots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|slot| slot["id"] == "body")
        .unwrap();
    let cta2 = second["plan"]["slots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|slot| slot["id"] == "cta")
        .unwrap();
    assert_eq!(body2["cache"], "hit");
    assert_eq!(cta2["cache"], "hit");
    assert_eq!(body2["blob"], body_blob);
    assert_eq!(cta2["blob"], cta_blob);

    let export = dir.join("timeline.otio.json");
    let exported = run_ok(
        &dir,
        &[
            "export",
            "otio",
            "--scion",
            "9x16",
            "--out",
            export.to_str().unwrap(),
        ],
    );
    assert!(export.is_file());
    assert!(exported["loss"]["lost"].is_array());
    let imported = run_ok(
        &dir,
        &[
            "import",
            "otio",
            export.to_str().unwrap(),
            "--scion",
            "from-otio",
        ],
    );
    assert_eq!(imported["scion"], "from-otio");
    assert!(imported["loss"]["lost"].is_array());
    assert!(dir.join("scions/from-otio.json").is_file());
    assert!(dir.join("scions/from-otio.loss.json").is_file());

    run_ok(
        &dir,
        &["preview", "--scion", "9x16", "--out", "preview.mp4"],
    );
    assert!(dir.join("preview.mp4").is_file());
    assert!(probe_has_audio(&dir.join("preview.mp4")));
    assert!(probe_has_audio(&dir.join("ad.mp4")));
    let remote = dir.join("object-store");
    let pushed = run_ok(
        &dir,
        &["store", "push", "--remote", remote.to_str().unwrap()],
    );
    assert!(pushed["pushed"]
        .as_array()
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));
    let _ = fs::remove_dir_all(&dir);
}

fn probe_has_audio(path: &Path) -> bool {
    let ffprobe = std::env::var("FFPROBE").unwrap_or_else(|_| "ffprobe".into());
    Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=index",
            "-of",
            "csv=p=0",
            path.to_str().unwrap(),
        ])
        .output()
        .map(|out| out.status.success() && !out.stdout.is_empty())
        .unwrap_or(false)
}

#[test]
fn speed_retime_and_preview_audio_on_generated_media() {
    if !ffmpeg_available() {
        eprintln!("skip speed_retime_and_preview_audio_on_generated_media — install ffmpeg");
        return;
    }
    let dir = tmp("retime-preview");
    run_ok(&dir, &["init"]);
    run_ok(&dir, &["slot", "hook", "--span", "0-1", "--window", "0-1"]);
    run_ok(&dir, &["slot", "body", "--span", "1-3"]);
    run_ok(
        &dir,
        &[
            "scion",
            "create",
            "ad",
            "--dest",
            "64x64",
            "--encoder",
            "x264",
        ],
    );
    let hook = write_color(&dir, "hook.mp4", "red", "2.2", "10");
    let body = write_color(&dir, "body.mp4", "green", "2.2", "10");
    run_ok(
        &dir,
        &[
            "bind",
            "hook",
            hook.to_str().unwrap(),
            "--in",
            "0",
            "--out",
            "1",
        ],
    );
    run_ok(
        &dir,
        &[
            "bind",
            "body",
            body.to_str().unwrap(),
            "--in",
            "0",
            "--out",
            "2",
        ],
    );
    let first = run_ok(&dir, &["compile", "--out", "one.mp4"]);
    let warmed = run_ok(&dir, &["compile", "--out", "one-warm.mp4"]);
    let body_blob = warmed["plan"]["slots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|slot| slot["id"] == "body")
        .unwrap()["blob"]
        .clone();
    run_ok(
        &dir,
        &[
            "bind",
            "hook",
            hook.to_str().unwrap(),
            "--in",
            "0",
            "--out",
            "2",
            "--speed",
            "2",
        ],
    );
    let second = run_ok(&dir, &["compile", "--out", "sped.mp4"]);
    let hook2 = second["plan"]["slots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|slot| slot["id"] == "hook")
        .unwrap();
    let body2 = second["plan"]["slots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|slot| slot["id"] == "body")
        .unwrap();
    assert_eq!(hook2["cache"], "miss");
    assert_eq!(body2["cache"], "hit");
    assert_eq!(body2["blob"], body_blob);
    run_ok(&dir, &["preview", "--out", "preview.mp4"]);
    assert!(dir.join("preview.mp4").is_file());
    assert!(probe_has_audio(&dir.join("preview.mp4")));
    assert!(probe_has_audio(&dir.join("sped.mp4")));
    assert_eq!(first["plan"]["encode"], true);
    let _ = fs::remove_dir_all(&dir);
}
