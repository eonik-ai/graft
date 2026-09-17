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

fn write_sine(dir: &Path, name: &str, seconds: &str) -> PathBuf {
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
            &format!("sine=frequency=440:duration={seconds}"),
            "-c:a",
            "pcm_s16le",
            path.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "ffmpeg sine {name}");
    path
}

fn copy_video_range(dir: &Path, src: &str, start: &str, duration: &str, dest: &str) -> Vec<u8> {
    let ffmpeg = std::env::var("FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
    let path = dir.join(dest);
    let status = Command::new(&ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-i",
            dir.join(src).to_str().unwrap(),
            "-ss",
            start,
            "-t",
            duration,
            "-an",
            "-c:v",
            "copy",
            "-f",
            "h264",
            path.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(
        status.success(),
        "ffmpeg copy range {src} {start}+{duration}"
    );
    fs::read(&path).unwrap()
}

#[test]
fn version_prints_crate_version() {
    let output = graft().arg("--version").output().expect("graft --version");
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains(env!("CARGO_PKG_VERSION")),
        "version stdout: {text}"
    );
}

fn run_ok(dir: &Path, args: &[&str]) -> serde_json::Value {
    run_ok_full(dir, args).0
}

fn run_ok_full(dir: &Path, args: &[&str]) -> (serde_json::Value, String) {
    let output = graft()
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("run graft");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "graft {} failed\nstdout: {}\nstderr: {stderr}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
    );
    let json = if output.stdout.is_empty() {
        serde_json::json!(null)
    } else {
        serde_json::from_slice(&output.stdout).unwrap_or(serde_json::json!(null))
    };
    (json, stderr)
}

fn run_fail(dir: &Path, args: &[&str]) -> String {
    let output = graft()
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("run graft");
    assert!(
        !output.status.success(),
        "graft {} should fail\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn dub_example_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/dub-en-9x16")
}

fn copy_dub_example() -> PathBuf {
    let src = dub_example_dir();
    let dir = tmp("dub-example");
    for entry in fs::read_dir(&src).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            fs::copy(&path, dir.join(path.file_name().unwrap())).unwrap();
        }
    }
    fs::create_dir_all(dir.join("scions")).unwrap();
    for entry in fs::read_dir(src.join("scions")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            fs::copy(&path, dir.join("scions").join(path.file_name().unwrap())).unwrap();
        }
    }
    dir
}

fn seed_build_from_json(dir: &Path) -> String {
    let build: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("build.json")).unwrap()).unwrap();
    let id = build["id"].as_str().unwrap().to_string();
    let root = dir.join(".graft/builds").join(&id);
    fs::create_dir_all(&root).unwrap();
    fs::copy(dir.join("build.json"), root.join("build.json")).unwrap();
    id
}

fn assert_signal_matches_fixture(got: &serde_json::Value, expected: &serde_json::Value) {
    assert_eq!(got["signal"]["kind"], expected["signal"]["kind"]);
    assert_eq!(got["signal"]["range"], expected["signal"]["range"]);
    assert_eq!(got["slots"], expected["slots"]);
    assert_eq!(got["kerfs"], expected["kerfs"]);
    assert_eq!(got["clean"], expected["clean"]);
    let mixes = expected
        .get("mixes")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let got_mixes = got
        .get("mixes")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    assert_eq!(got_mixes, mixes);
}

const DUB_SIGNAL_CASES: &[(&str, Option<&str>, &str)] = &[
    ("note", Some("0-3"), "dirty-note-0-3.json"),
    ("note", Some("3-5"), "dirty-note.json"),
    ("note", Some("5-6"), "dirty-note-5-6.json"),
    ("note", Some("7-10"), "dirty-note-7-10.json"),
    ("note", Some("2.5-3.5"), "dirty-note-join-hook-body.json"),
    ("note", Some("6.9-7.1"), "dirty-note-join-body-cta.json"),
    ("note", Some("0-10"), "dirty-note-full.json"),
    ("note", Some("12-13"), "dirty-note-past.json"),
    (
        "note",
        Some("2.966666666666667-3"),
        "dirty-note-frame-89.json",
    ),
    (
        "note",
        Some("3-3.033333333333333"),
        "dirty-note-frame-90.json",
    ),
    ("vo_hold", Some("8-9"), "dirty.json"),
    ("vo_hold", Some("0-1"), "dirty.json"),
    ("vo_hold", None, "dirty.json"),
    ("hold", Some("0-1"), "dirty-hold.json"),
    ("hold", Some("3-4"), "dirty-hold.json"),
    ("hold", None, "dirty-hold.json"),
    ("hook_rate", Some("3-5"), "dirty-hook-rate.json"),
    ("hook_rate", Some("0-1"), "dirty-hook-rate.json"),
    ("hook_rate", None, "dirty-hook-rate.json"),
];

fn load_dub_fixture(name: &str) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(dub_example_dir().join(name)).unwrap()).unwrap()
}

fn assert_dub_signal_table(dir: &Path, build: &str) {
    for (kind, t, file) in DUB_SIGNAL_CASES {
        let mut args = vec!["signal", "--kind", kind, "--build", build];
        if let Some(span) = t {
            args.extend(["--t", span]);
        }
        let got = run_ok(dir, &args);
        let expected = load_dub_fixture(file);
        assert_signal_matches_fixture(&got, &expected);
        assert!(
            !got["slots"]
                .as_array()
                .unwrap()
                .iter()
                .any(|slot| slot == "bed"),
            "{kind} {t:?} {file}"
        );
        if *kind == "hold" {
            assert_eq!(
                json_strings(&got["slots"]),
                vec!["body", "captions", "vo"],
                "{kind} {t:?}"
            );
            assert!(!json_strings(&got["slots"]).contains(&"hook".to_string()));
        }
        if matches!(*kind, "vo_hold" | "hook_rate") {
            assert!(
                !json_strings(&got["slots"]).contains(&"body".to_string()),
                "{kind}"
            );
            assert_eq!(
                json_strings(&got["slots"]),
                vec!["hook", "captions", "vo"],
                "{kind} {t:?}"
            );
        }
    }
}

fn dub_note_slots_for_frame(start: i64) -> Vec<&'static str> {
    if !(0..300).contains(&start) {
        return vec![];
    }
    let spine = if start < 90 {
        "hook"
    } else if start < 210 {
        "body"
    } else {
        "cta"
    };
    vec![spine, "captions", "vo"]
}

fn ingest_every_dest_frame(dir: &Path, build: &str) {
    let mut items = Vec::with_capacity(1200);
    for start in 0..300i64 {
        items.push(serde_json::json!({
            "id": format!("n{start}"),
            "build": build,
            "kind": "note",
            "range": { "start": start, "duration": 1 }
        }));
        items.push(serde_json::json!({
            "id": format!("vh{start}"),
            "build": build,
            "kind": "vo_hold",
            "range": { "start": start, "duration": 1 }
        }));
        items.push(serde_json::json!({
            "id": format!("h{start}"),
            "build": build,
            "kind": "hold",
            "range": { "start": start, "duration": 1 }
        }));
        items.push(serde_json::json!({
            "id": format!("hr{start}"),
            "build": build,
            "kind": "hook_rate",
            "range": { "start": start, "duration": 1 }
        }));
    }
    fs::write(
        dir.join("all-frames.json"),
        serde_json::to_string(&items).unwrap(),
    )
    .unwrap();
    let written = run_ok(dir, &["feedback", "ingest", "all-frames.json"]);
    assert_eq!(written["written"].as_array().unwrap().len(), 1200);
    let vo_hold = load_dub_fixture("dirty.json");
    let hold = load_dub_fixture("dirty-hold.json");
    let hook_rate = load_dub_fixture("dirty-hook-rate.json");
    for start in 0..300i64 {
        let note: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(dir.join(format!("feedback/n{start}.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(
            json_strings(&note["resolved"]["slots"]),
            dub_note_slots_for_frame(start),
            "note frame {start}"
        );
        assert_eq!(
            note["range"],
            serde_json::json!({"start": start, "duration": 1})
        );
        assert!(!json_strings(&note["resolved"]["slots"]).contains(&"bed".to_string()));
        let got_vo: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(dir.join(format!("feedback/vh{start}.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(
            got_vo["resolved"]["slots"], vo_hold["slots"],
            "vo_hold {start}"
        );
        assert_eq!(got_vo["range"], vo_hold["signal"]["range"]);
        let got_hold: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(dir.join(format!("feedback/h{start}.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(got_hold["resolved"]["slots"], hold["slots"], "hold {start}");
        assert_eq!(got_hold["range"], hold["signal"]["range"]);
        let got_hook: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(dir.join(format!("feedback/hr{start}.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(
            got_hook["resolved"]["slots"], hook_rate["slots"],
            "hook_rate {start}"
        );
        assert_eq!(got_hook["range"], hook_rate["signal"]["range"]);
    }
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
fn signal_dub_permutations_match_fixtures() {
    let dir = copy_dub_example();
    let build = seed_build_from_json(&dir);
    assert_dub_signal_table(&dir, &build);
    let err = run_fail(&dir, &["signal", "--kind", "note", "--build", &build]);
    assert!(
        err.contains("pass --t") || err.contains("no declared window"),
        "{err}"
    );
    ingest_every_dest_frame(&dir, &build);
    let expected_89 = load_dub_fixture("dirty-note-frame-89.json");
    let expected_90 = load_dub_fixture("dirty-note-frame-90.json");
    let fb89: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("feedback/n89.json")).unwrap()).unwrap();
    let fb90: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("feedback/n90.json")).unwrap()).unwrap();
    assert_eq!(fb89["resolved"]["slots"], expected_89["slots"]);
    assert_eq!(fb89["range"], expected_89["signal"]["range"]);
    assert_eq!(fb90["resolved"]["slots"], expected_90["slots"]);
    assert_eq!(fb90["range"], expected_90["signal"]["range"]);
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

fn git(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(args)
        .output()
        .expect("run git")
}

fn git_ok(dir: &Path, args: &[&str]) {
    let output = git(dir, args);
    assert!(
        output.status.success(),
        "git {} failed\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_stdout(dir: &Path, args: &[&str]) -> String {
    let output = git(dir, args);
    assert!(
        output.status.success(),
        "git {} failed\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn git_init_local(dir: &Path) {
    let template = dir.join(".empty-git-template");
    fs::create_dir_all(&template).unwrap();
    git_ok(
        dir,
        &[
            "init",
            "--template",
            template.to_str().expect("utf8 template path"),
        ],
    );
    let _ = fs::remove_dir_all(&template);
    git_ok(dir, &["config", "user.email", "graft-test@invalid"]);
    git_ok(dir, &["config", "user.name", "graft test"]);
    git_ok(dir, &["config", "commit.gpgsign", "false"]);
    fs::write(dir.join(".gitignore"), ".graft/\n*.mp4\n*.mov\n*.mxf\n").unwrap();
}

fn git_add_commit(dir: &Path, message: &str) {
    git_ok(dir, &["add", "-A"]);
    git_ok(dir, &["commit", "-m", message]);
}

fn git_tracked_names(dir: &Path) -> Vec<String> {
    git_stdout(dir, &["ls-files"])
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn git_history_names(dir: &Path) -> Vec<String> {
    git_stdout(dir, &["log", "--name-only", "--pretty=format:"])
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn load_scion_json(dir: &Path, id: &str) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(dir.join(format!("scions/{id}.json"))).unwrap())
        .unwrap()
}

/// Git owns recipe revisions. Scion parentage is variant derivation. Builds
/// and CAS stay out of Git. iterate names a slot; it does not invent a take.
#[test]
fn founding_loop_history_is_not_an_mp4() {
    let dir = tmp("founding-loop");
    git_init_local(&dir);

    run_ok(&dir, &["init"]);
    run_ok(&dir, &["slot", "body", "--span", "3-20"]);
    run_ok(
        &dir,
        &[
            "scion",
            "create",
            "picture",
            "--dest",
            "1080x1920",
            "--encoder",
            "graft-intra",
        ],
    );
    let hook = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let body = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let hook_v2 = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    let body_v2 = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    run_ok(
        &dir,
        &[
            "bind", "hook", hook, "--in", "0", "--out", "3", "--scion", "picture",
        ],
    );
    run_ok(
        &dir,
        &[
            "bind", "body", body, "--in", "0", "--out", "17", "--scion", "picture",
        ],
    );

    let status = run_ok(&dir, &["status"]);
    assert_eq!(status["git_repository"], true);
    assert!(status["recipe_files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f == "score.json"));
    assert!(status["recipe_files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f == "scions/picture.json"));
    assert!(status["note"]
        .as_str()
        .unwrap()
        .contains("does not replace git"));
    assert!(!status["commit_ready"].as_array().unwrap().is_empty());

    git_add_commit(&dir, "score and picture scion");
    let after_commit = run_ok(&dir, &["status"]);
    assert!(after_commit["commit_ready"].as_array().unwrap().is_empty());
    let tracked = git_tracked_names(&dir);
    assert!(tracked.iter().any(|f| f == "score.json"));
    assert!(tracked.iter().any(|f| f == "scions/picture.json"));
    assert!(tracked.iter().any(|f| f == ".gitignore"));
    assert!(!tracked.iter().any(|f| f.starts_with(".graft")));
    assert!(!tracked.iter().any(|f| f.ends_with(".mp4")));

    let compile = run_ok(&dir, &["compile", "--scion", "picture"]);
    assert_eq!(compile["plan"]["encode"], false);
    let picture_build = compile["build"]["id"].as_str().unwrap().to_string();
    let picture_map = compile["build"]["time_map"].as_str().unwrap().to_string();
    assert!(dir
        .join(".graft/builds")
        .join(&picture_build)
        .join("build.json")
        .is_file());
    assert!(dir.join(&picture_map).is_file());
    assert!(picture_map.contains(".graft/"));
    git_ok(&dir, &["add", "-A"]);
    let porcelain = git_stdout(&dir, &["status", "--porcelain"]);
    assert!(
        porcelain.trim().is_empty(),
        "compile must not dirty Git recipes\n{porcelain}"
    );
    let after_compile = git_tracked_names(&dir);
    assert!(!after_compile.iter().any(|f| f.starts_with(".graft")));
    assert!(!after_compile.iter().any(|f| f.contains("time-map")));
    assert_eq!(after_compile, tracked);

    run_ok(&dir, &["scion", "fork", "picture", "hook-v2"]);
    run_ok(
        &dir,
        &[
            "bind", "hook", hook_v2, "--in", "0", "--out", "3", "--scion", "hook-v2",
        ],
    );
    let hook_child = load_scion_json(&dir, "hook-v2");
    assert_eq!(hook_child["parent"], "picture");
    assert_eq!(
        hook_child["layers"][0]["bindings"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        vec!["hook"]
    );
    let hook_diff = run_ok(&dir, &["diff", "picture", "hook-v2"]);
    assert_eq!(diff_paths(&hook_diff), vec!["slots.hook.binding"]);
    git_add_commit(&dir, "fork hook-v2");
    let hook_commit_parent = git_stdout(&dir, &["log", "-1", "--format=%P"])
        .trim()
        .to_string();
    assert!(!hook_commit_parent.is_empty());
    assert_ne!(hook_commit_parent, "picture");

    run_ok(&dir, &["scion", "fork", "picture", "body-v2"]);
    run_ok(
        &dir,
        &[
            "bind", "body", body_v2, "--in", "0", "--out", "17", "--scion", "body-v2",
        ],
    );
    let merged = run_ok(
        &dir,
        &["merge", "picture", "hook-v2", "body-v2", "--id", "merged"],
    );
    assert!(merged["merged"].is_object());
    assert_eq!(merged["conflicts"], serde_json::json!([]));
    let vs_hook = run_ok(&dir, &["diff", "hook-v2", "merged"]);
    assert_eq!(diff_paths(&vs_hook), vec!["slots.body.binding"]);
    let vs_body = run_ok(&dir, &["diff", "body-v2", "merged"]);
    assert_eq!(diff_paths(&vs_body), vec!["slots.hook.binding"]);
    git_add_commit(&dir, "merge hook-v2 and body-v2");

    let hooked = run_ok(&dir, &["compile", "--scion", "hook-v2"]);
    let hook_build = hooked["build"]["id"].as_str().unwrap().to_string();
    assert_ne!(hook_build, picture_build);
    assert!(dir
        .join(".graft/builds")
        .join(&picture_build)
        .join("build.json")
        .is_file());
    assert!(dir
        .join(".graft/builds")
        .join(&hook_build)
        .join("build.json")
        .is_file());

    let signal = run_ok(
        &dir,
        &["signal", "--kind", "hook_rate", "--build", &picture_build],
    );
    assert_eq!(json_strings(&signal["slots"]), vec!["hook"]);
    assert!(json_strings(&signal["clean"]).contains(&"body".to_string()));
    assert!(!json_strings(&signal["slots"]).contains(&"body".to_string()));

    fs::write(
        dir.join("hook-rate.json"),
        serde_json::json!({
            "id": "fb-hook",
            "build": picture_build,
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
            &picture_build,
            "--feedback",
            "fb-hook",
            "--scion",
            "hook-v3",
        ],
    );
    assert_eq!(iterate["dirty_slots"], serde_json::json!(["hook"]));
    assert_eq!(iterate["creative_replacement"], "required");
    assert_eq!(iterate["parent"], "picture");
    let child = load_scion_json(&dir, "hook-v3");
    assert_eq!(child["parent"], "picture");
    assert!(child["layers"].as_array().unwrap().is_empty());
    assert_eq!(child["change_request"]["feedback"], "fb-hook");
    assert_eq!(
        child["change_request"]["slots"],
        serde_json::json!(["hook"])
    );
    assert!(!serde_json::to_string(&child).unwrap().contains("speed"));

    git_add_commit(&dir, "signal iterate hook-v3");
    let history_names = git_history_names(&dir);
    assert!(history_names.iter().any(|f| f == "score.json"));
    assert!(history_names.iter().any(|f| f == "scions/hook-v3.json"));
    assert!(history_names.iter().any(|f| f.contains("feedback")));
    assert!(!history_names.iter().any(|f| f.starts_with(".graft")));
    assert!(!history_names.iter().any(|f| f.ends_with(".mp4")));
    let log = git_stdout(&dir, &["log", "--oneline"]);
    assert!(log.lines().count() >= 3);

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

fn probe_duration_s(path: &Path) -> f64 {
    let ffprobe = std::env::var("FFPROBE").unwrap_or_else(|_| "ffprobe".into());
    let output = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("ffprobe duration");
    assert!(
        output.status.success(),
        "ffprobe duration {}",
        path.display()
    );
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<f64>()
        .expect("duration float")
}

fn json_strings(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap().to_string())
        .collect()
}

fn load_build_time_map(dir: &Path, compile: &serde_json::Value) -> serde_json::Value {
    let rel = compile["build"]["time_map"].as_str().unwrap();
    serde_json::from_str(&fs::read_to_string(dir.join(rel)).unwrap()).unwrap()
}

fn time_map_slot_ids(map: &serde_json::Value) -> Vec<String> {
    map["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["slot"].as_str().unwrap().to_string())
        .collect()
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

#[test]
fn scion_create_defaults_dest_id_from_score() {
    let dir = tmp("dest-default");
    run_ok(&dir, &["init"]);
    run_ok(
        &dir,
        &[
            "scion",
            "create",
            "phone",
            "--dest",
            "64x64",
            "--encoder",
            "graft-intra",
        ],
    );
    let scion: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("scions/phone.json")).unwrap()).unwrap();
    assert_eq!(scion["dest"]["id"], "9x16");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn overlay_roles_write_sidecar_and_keep_body_on_hook_swap() {
    if !ffmpeg_available() {
        eprintln!("skip overlay_roles_write_sidecar_and_keep_body_on_hook_swap — install ffmpeg");
        return;
    }
    let dir = tmp("overlay");
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
    run_ok(&dir, &["slot", "vo", "--span", "0-1", "--role", "vo"]);
    run_ok(&dir, &["slot", "bed", "--span", "0-4", "--role", "bed"]);
    run_ok(
        &dir,
        &["slot", "captions", "--span", "0-4", "--role", "captions"],
    );
    run_ok(&dir, &["slot", "brand", "--span", "0-4", "--role", "brand"]);
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
    let vo = write_color(&dir, "vo.mp4", "gray", "1.2", "30");
    let bed = write_color(&dir, "bed.mp4", "black", "4.2", "30");
    let brand = write_color(&dir, "brand.mp4", "white", "1.2", "30");
    let captions = dir.join("caps.vtt");
    fs::write(&captions, "WEBVTT\n\n00:00.000 --> 00:04.000\nHi\n").unwrap();
    let bind = |slot: &str, file: &Path, out: &str| {
        run_ok(
            &dir,
            &[
                "bind",
                slot,
                file.to_str().unwrap(),
                "--in",
                "0",
                "--out",
                out,
                "--scion",
                "9x16",
            ],
        );
    };
    bind("hook", &hook_a, "1");
    bind("body", &body, "2");
    bind("cta", &cta, "1");
    bind("vo", &vo, "1");
    bind("bed", &bed, "4");
    bind("captions", &captions, "4");
    bind("brand", &brand, "4");
    let first = run_ok(&dir, &["compile", "--scion", "9x16", "--out", "ad.mp4"]);
    assert_eq!(first["plan"]["encode"], true);
    assert!(dir.join("ad.mp4").is_file());
    assert!(dir.join("ad.vtt").is_file());
    assert!(probe_has_audio(&dir.join("ad.mp4")));
    let warmed = run_ok(
        &dir,
        &["compile", "--scion", "9x16", "--out", "ad-warm.mp4"],
    );
    let body_blob = warmed["plan"]["slots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|slot| slot["id"] == "body")
        .unwrap()["blob"]
        .clone();
    assert!(body_blob.is_string());
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
            "9x16",
        ],
    );
    let second = run_ok(&dir, &["compile", "--scion", "9x16", "--out", "ad-v2.mp4"]);
    let body_after = second["plan"]["slots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|slot| slot["id"] == "body")
        .unwrap()["blob"]
        .clone();
    assert_eq!(body_blob, body_after);
    assert_eq!(
        second["plan"]["slots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|slot| slot["id"] == "body")
            .unwrap()["cache"],
        "hit"
    );
    let dirty = run_ok(&dir, &["dirty", "--scion", "9x16"]);
    assert!(dirty["clean"]
        .as_array()
        .unwrap()
        .iter()
        .any(|slot| slot == "body"));
    let _ = fs::remove_dir_all(&dir);
}

fn plan_slot<'a>(compile: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    compile["plan"]["slots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|slot| slot["id"] == id)
        .unwrap_or_else(|| panic!("missing slot {id}"))
}

fn plan_caption<'a>(compile: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    compile["plan"]["captions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|slot| slot["id"] == id)
        .unwrap_or_else(|| panic!("missing captions {id}"))
}

fn plan_overlay_audio<'a>(compile: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    compile["plan"]["overlay_audio"]
        .as_array()
        .unwrap()
        .iter()
        .find(|slot| slot["id"] == id)
        .unwrap_or_else(|| panic!("missing overlay_audio {id}"))
}

fn diff_paths(diff: &serde_json::Value) -> Vec<String> {
    diff.as_array()
        .unwrap()
        .iter()
        .map(|change| change["path"].as_str().unwrap().to_string())
        .collect()
}

fn probe_wh(path: &Path) -> (u32, u32) {
    let ffprobe = std::env::var("FFPROBE").unwrap_or_else(|_| "ffprobe".into());
    let output = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("ffprobe size");
    assert!(output.status.success(), "ffprobe size {}", path.display());
    let text = String::from_utf8_lossy(&output.stdout);
    let mut parts = text.trim().split(',');
    let width = parts.next().unwrap().parse().expect("width");
    let height = parts.next().unwrap().parse().expect("height");
    (width, height)
}

fn eonik_file(parts: &[&str]) -> Option<PathBuf> {
    let mut path = PathBuf::from(std::env::var("HOME").ok()?);
    path.push("eonik");
    for part in parts {
        path.push(part);
    }
    path.is_file().then_some(path)
}

struct RealClips {
    hook: PathBuf,
    hook_b: PathBuf,
    body: PathBuf,
    body_b: PathBuf,
    cta: PathBuf,
    vo_en: PathBuf,
    vo_en2: PathBuf,
}

fn real_clips() -> Option<RealClips> {
    let veo_a = "eonik_ads_script/artifacts/7cc2ecdd-472f-4463-98a4-a1830bb32eb9/veo";
    let veo_b = "eonik_ads_script/artifacts/d38c5a4f-679f-4e89-83a0-39e355f869c5/veo";
    Some(RealClips {
        hook: eonik_file(&[veo_a, "clip_01.mp4"])?,
        body: eonik_file(&[veo_a, "clip_02.mp4"])?,
        cta: eonik_file(&[veo_a, "clip_03.mp4"])?,
        hook_b: eonik_file(&[veo_b, "clip_01.mp4"])?,
        body_b: eonik_file(&[veo_b, "clip_02.mp4"])?,
        vo_en: eonik_file(&["mywonder_clip1.mov"])?,
        vo_en2: eonik_file(&["video2.mov"])?,
    })
}

/// Captions sidecar timed to the dest spans cut from clip_01/02/03.
const CLIP_CAPTIONS_VTT: &str = "WEBVTT\n\n00:00.000 --> 00:03.000\nclip_01\n\n00:03.000 --> 00:07.000\nclip_02\n\n00:07.000 --> 00:10.000\nclip_03\n";

/// Dub inherit, approver 3–5s, hold/too-slow, and a disjoint hook fork on real clips.
#[test]
fn three_cases_on_one_clock() {
    if !ffmpeg_available() {
        eprintln!("skip three_cases_on_one_clock — install ffmpeg");
        return;
    }
    let Some(clips) = real_clips() else {
        eprintln!("skip three_cases_on_one_clock — missing ~/eonik Veo/MyWonder clips");
        return;
    };
    let dir = tmp("three-cases");
    run_ok(&dir, &["init"]);
    run_ok(
        &dir,
        &[
            "slot",
            "body",
            "--span",
            "3-7",
            "--window",
            "3-7",
            "--window-kind",
            "hold",
        ],
    );
    run_ok(
        &dir,
        &[
            "slot",
            "cta",
            "--span",
            "7-10",
            "--role",
            "cta",
            "--optional",
        ],
    );
    run_ok(
        &dir,
        &[
            "slot",
            "vo",
            "--span",
            "0-10",
            "--role",
            "vo",
            "--window",
            "0-3",
            "--window-kind",
            "vo_hold",
            "--optional",
        ],
    );
    run_ok(
        &dir,
        &[
            "slot",
            "captions",
            "--span",
            "0-10",
            "--role",
            "captions",
            "--optional",
        ],
    );
    run_ok(
        &dir,
        &[
            "scion",
            "create",
            "picture",
            "--dest",
            "720x1280",
            "--encoder",
            "x264",
        ],
    );
    let captions = dir.join("caps.vtt");
    fs::write(&captions, CLIP_CAPTIONS_VTT).unwrap();
    let hook_a = &clips.hook;
    let hook_b = &clips.hook_b;
    let body = &clips.body;
    let body_b = &clips.body_b;
    let cta = &clips.cta;
    let vo_en = &clips.vo_en;
    let vo_hi = &clips.vo_en2;
    assert_ne!(vo_en, vo_hi);
    let bind_picture = |slot: &str, file: &Path, out: &str| {
        run_ok(
            &dir,
            &[
                "bind",
                slot,
                file.to_str().unwrap(),
                "--in",
                "0",
                "--out",
                out,
                "--scion",
                "picture",
            ],
        );
    };
    bind_picture("hook", hook_a, "3");
    bind_picture("body", body, "4");
    bind_picture("cta", cta, "3");
    bind_picture("captions", &captions, "10");
    let picture = run_ok(
        &dir,
        &["compile", "--scion", "picture", "--out", "picture.mp4"],
    );
    assert_eq!(picture["plan"]["encode"], true);
    assert_eq!(plan_slot(&picture, "body")["cache"], "miss");
    let warmed = run_ok(
        &dir,
        &["compile", "--scion", "picture", "--out", "picture-warm.mp4"],
    );
    assert_eq!(plan_slot(&warmed, "body")["cache"], "hit");
    let body_blob = plan_slot(&warmed, "body")["blob"].clone();
    assert!(body_blob.is_string());
    let hook_blob = plan_slot(&warmed, "hook")["blob"].clone();
    let cta_blob = plan_slot(&warmed, "cta")["blob"].clone();
    assert!((probe_duration_s(&dir.join("picture.mp4")) - 10.0).abs() < 0.05);
    assert_eq!(probe_wh(&dir.join("picture.mp4")), (720, 1280));
    assert!(probe_has_audio(&dir.join("picture.mp4")));
    assert!(dir.join("picture.vtt").is_file());
    let picture_map = load_build_time_map(&dir, &picture);
    assert_eq!(
        time_map_slot_ids(&picture_map),
        vec!["hook", "body", "cta", "captions"]
    );
    assert_eq!(
        picture_map["entries"][0]["dest"],
        serde_json::json!({"start": 0, "duration": 90})
    );
    assert_eq!(
        picture_map["entries"][1]["dest"],
        serde_json::json!({"start": 90, "duration": 120})
    );
    assert_eq!(
        picture_map["entries"][2]["dest"],
        serde_json::json!({"start": 210, "duration": 90})
    );
    let picture_build = picture["build"]["id"].as_str().unwrap().to_string();
    let pic_note = run_ok(
        &dir,
        &[
            "signal",
            "--kind",
            "note",
            "--t",
            "3-5",
            "--build",
            &picture_build,
        ],
    );
    assert_eq!(json_strings(&pic_note["slots"]), vec!["body", "captions"]);
    assert_eq!(json_strings(&pic_note["clean"]), vec!["hook", "cta"]);
    assert_eq!(
        pic_note
            .get("mixes")
            .cloned()
            .unwrap_or(serde_json::json!([])),
        serde_json::json!([])
    );
    let pic_vo = run_ok(
        &dir,
        &["signal", "--kind", "vo_hold", "--build", &picture_build],
    );
    assert_eq!(json_strings(&pic_vo["slots"]), vec!["hook", "captions"]);
    assert!(!json_strings(&pic_vo["slots"]).contains(&"vo".to_string()));
    assert!(!json_strings(&pic_vo["slots"]).contains(&"body".to_string()));
    let pic_hold_t0 = run_ok(
        &dir,
        &[
            "signal",
            "--kind",
            "hold",
            "--t",
            "0-1",
            "--build",
            &picture_build,
        ],
    );
    assert_eq!(
        pic_hold_t0["signal"]["range"],
        serde_json::json!({"start": 90, "duration": 120})
    );
    assert_eq!(
        json_strings(&pic_hold_t0["slots"]),
        vec!["body", "captions"]
    );
    let pic_hook_mid = run_ok(
        &dir,
        &[
            "signal",
            "--kind",
            "hook_rate",
            "--t",
            "3-5",
            "--build",
            &picture_build,
        ],
    );
    assert_eq!(
        pic_hook_mid["signal"]["range"],
        serde_json::json!({"start": 0, "duration": 90})
    );
    assert!(!json_strings(&pic_hook_mid["slots"]).contains(&"body".to_string()));

    run_ok(&dir, &["scion", "fork", "picture", "dub-en"]);
    run_ok(
        &dir,
        &[
            "bind",
            "vo",
            vo_en.to_str().unwrap(),
            "--in",
            "0",
            "--out",
            "10",
            "--scion",
            "dub-en",
            "--layer",
            "copy",
        ],
    );
    let dub = run_ok(
        &dir,
        &["compile", "--scion", "dub-en", "--out", "dub-en.mp4"],
    );
    assert_eq!(plan_slot(&dub, "body")["cache"], "hit");
    assert_eq!(plan_slot(&dub, "body")["blob"], body_blob);
    assert_eq!(plan_slot(&dub, "hook")["cache"], "hit");
    assert_eq!(plan_slot(&dub, "hook")["blob"], hook_blob);
    assert_eq!(plan_slot(&dub, "cta")["cache"], "hit");
    assert_eq!(plan_slot(&dub, "cta")["blob"], cta_blob);
    assert_eq!(dub["plan"]["audio_mix"], "miss");
    assert_eq!(dub["plan"]["concat_cache"], "miss");
    assert_eq!(plan_caption(&dub, "captions")["cache"], "hit");
    let vo_audio = plan_overlay_audio(&dub, "vo");
    assert_eq!(vo_audio["cache"], "miss");
    assert!((probe_duration_s(&dir.join("dub-en.mp4")) - 10.0).abs() < 0.05);
    assert!(probe_has_audio(&dir.join("dub-en.mp4")));
    assert!(dir.join("dub-en.vtt").is_file());
    let dub_map = load_build_time_map(&dir, &dub);
    assert_eq!(
        time_map_slot_ids(&dub_map),
        vec!["hook", "body", "cta", "captions", "vo"]
    );

    run_ok(
        &dir,
        &[
            "bind",
            "vo",
            vo_hi.to_str().unwrap(),
            "--in",
            "0",
            "--out",
            "10",
            "--scion",
            "dub-en",
            "--layer",
            "copy",
        ],
    );
    let dub2 = run_ok(
        &dir,
        &["compile", "--scion", "dub-en", "--out", "dub-en2.mp4"],
    );
    assert_eq!(plan_slot(&dub2, "body")["cache"], "hit");
    assert_eq!(plan_slot(&dub2, "body")["blob"], body_blob);
    assert_eq!(plan_slot(&dub2, "hook")["cache"], "hit");
    assert_eq!(plan_slot(&dub2, "cta")["cache"], "hit");
    assert_eq!(dub2["plan"]["audio_mix"], "miss");
    assert_eq!(plan_caption(&dub2, "captions")["cache"], "hit");
    let vo_audio2 = plan_overlay_audio(&dub2, "vo");
    assert_eq!(vo_audio2["cache"], "miss");
    assert_ne!(vo_audio2["slot_encode"], vo_audio["slot_encode"]);
    let dub_build = dub2["build"]["id"].as_str().unwrap().to_string();

    let diff = run_ok(&dir, &["diff", "picture", "dub-en"]);
    let paths: Vec<&str> = diff
        .as_array()
        .unwrap()
        .iter()
        .map(|change| change["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths, vec!["slots.vo.binding"]);

    assert_dub_signal_table(&dir, &dub_build);
    let note_err = run_fail(&dir, &["signal", "--kind", "note", "--build", &dub_build]);
    assert!(
        note_err.contains("pass --t") || note_err.contains("no declared window"),
        "{note_err}"
    );
    fs::write(
        dir.join("note-3-5.json"),
        serde_json::json!({
            "id": "fb-note",
            "build": dub_build,
            "kind": "note",
            "range": { "start": 90, "duration": 60 }
        })
        .to_string(),
    )
    .unwrap();
    run_ok(&dir, &["feedback", "ingest", "note-3-5.json"]);
    let iterate_note = run_ok(
        &dir,
        &[
            "iterate",
            "--from",
            &dub_build,
            "--feedback",
            "fb-note",
            "--scion",
            "note-3-5",
        ],
    );
    assert_eq!(iterate_note["creative_replacement"], "required");
    assert_eq!(
        json_strings(&iterate_note["dirty_slots"]),
        vec!["body", "captions", "vo"]
    );
    let note_scion: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("scions/note-3-5.json")).unwrap())
            .unwrap();
    assert!(note_scion["layers"].as_array().unwrap().is_empty());
    assert_eq!(note_scion["parent"], "dub-en");
    assert_eq!(
        note_scion["change_request"]["slots"],
        iterate_note["dirty_slots"]
    );
    assert!(!serde_json::to_string(&note_scion)
        .unwrap()
        .contains("speed"));
    let empty_note = run_ok(
        &dir,
        &["compile", "--scion", "note-3-5", "--out", "note-empty.mp4"],
    );
    assert_eq!(plan_slot(&empty_note, "body")["cache"], "hit");
    assert_eq!(plan_slot(&empty_note, "body")["blob"], body_blob);
    assert_eq!(plan_slot(&empty_note, "hook")["cache"], "hit");
    assert_eq!(plan_slot(&empty_note, "cta")["cache"], "hit");
    assert_eq!(plan_caption(&empty_note, "captions")["cache"], "hit");
    assert_eq!(plan_overlay_audio(&empty_note, "vo")["cache"], "hit");
    assert_eq!(empty_note["plan"]["audio_mix"], "hit");
    assert_eq!(empty_note["plan"]["concat_cache"], "hit");
    run_ok(
        &dir,
        &[
            "bind",
            "body",
            body_b.to_str().unwrap(),
            "--in",
            "0",
            "--out",
            "4",
            "--scion",
            "note-3-5",
        ],
    );
    let noted = run_ok(
        &dir,
        &["compile", "--scion", "note-3-5", "--out", "note.mp4"],
    );
    assert_eq!(plan_slot(&noted, "body")["cache"], "miss");
    assert_ne!(plan_slot(&noted, "body")["blob"], body_blob);
    assert_eq!(plan_slot(&noted, "hook")["cache"], "hit");
    assert_eq!(plan_slot(&noted, "cta")["cache"], "hit");
    assert_eq!(plan_caption(&noted, "captions")["cache"], "hit");
    assert_eq!(plan_overlay_audio(&noted, "vo")["cache"], "hit");

    let hold = run_ok(
        &dir,
        &[
            "signal",
            "--kind",
            "hold",
            "--t",
            "0-1",
            "--build",
            &picture_build,
        ],
    );
    assert_eq!(
        hold["signal"]["range"],
        serde_json::json!({"start": 90, "duration": 120})
    );
    assert_eq!(json_strings(&hold["slots"]), vec!["body", "captions"]);
    assert!(!json_strings(&hold["slots"]).contains(&"vo".to_string()));
    assert_eq!(
        hold.get("mixes").cloned().unwrap_or(serde_json::json!([])),
        serde_json::json!([])
    );
    fs::write(
        dir.join("hold.json"),
        serde_json::json!({
            "id": "fb-hold",
            "build": picture_build,
            "kind": "hold"
        })
        .to_string(),
    )
    .unwrap();
    run_ok(&dir, &["feedback", "ingest", "hold.json"]);
    let iterate_hold = run_ok(
        &dir,
        &[
            "iterate",
            "--from",
            &picture_build,
            "--feedback",
            "fb-hold",
            "--scion",
            "too-slow",
        ],
    );
    assert_eq!(iterate_hold["creative_replacement"], "required");
    assert_eq!(
        json_strings(&iterate_hold["dirty_slots"]),
        vec!["body", "captions"]
    );
    let hold_scion: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("scions/too-slow.json")).unwrap())
            .unwrap();
    assert!(hold_scion["layers"].as_array().unwrap().is_empty());
    assert_eq!(hold_scion["parent"], "picture");
    assert!(!serde_json::to_string(&hold_scion)
        .unwrap()
        .contains("speed"));
    let empty_hold = run_ok(
        &dir,
        &[
            "compile",
            "--scion",
            "too-slow",
            "--out",
            "too-slow-empty.mp4",
        ],
    );
    assert_eq!(plan_slot(&empty_hold, "body")["cache"], "hit");
    assert_eq!(plan_slot(&empty_hold, "body")["blob"], body_blob);
    assert_eq!(plan_slot(&empty_hold, "hook")["cache"], "hit");
    assert_eq!(plan_slot(&empty_hold, "cta")["cache"], "hit");
    run_ok(
        &dir,
        &[
            "bind",
            "body",
            body.to_str().unwrap(),
            "--in",
            "0",
            "--out",
            "8",
            "--speed",
            "2",
            "--scion",
            "too-slow",
        ],
    );
    let sped = run_ok(
        &dir,
        &["compile", "--scion", "too-slow", "--out", "sped.mp4"],
    );
    assert_eq!(plan_slot(&sped, "body")["cache"], "miss");
    assert_eq!(plan_slot(&sped, "hook")["cache"], "hit");
    assert_eq!(plan_slot(&sped, "cta")["cache"], "hit");
    assert_eq!(plan_caption(&sped, "captions")["cache"], "hit");
    let sped_scion: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("scions/too-slow.json")).unwrap())
            .unwrap();
    let sped_body = &sped_scion["layers"][0]["bindings"]["body"];
    assert_eq!(sped_body["params"]["speed"].as_f64(), Some(2.0));

    run_ok(&dir, &["scion", "fork", "picture", "hook-v2"]);
    run_ok(
        &dir,
        &[
            "bind",
            "hook",
            hook_b.to_str().unwrap(),
            "--in",
            "0",
            "--out",
            "3",
            "--scion",
            "hook-v2",
        ],
    );
    let hooked = run_ok(
        &dir,
        &["compile", "--scion", "hook-v2", "--out", "hook-v2.mp4"],
    );
    assert_eq!(plan_slot(&hooked, "body")["cache"], "hit");
    assert_eq!(plan_slot(&hooked, "body")["blob"], body_blob);
    assert_eq!(plan_slot(&hooked, "cta")["cache"], "hit");
    assert_eq!(plan_slot(&hooked, "hook")["cache"], "miss");
    let hook_diff = run_ok(&dir, &["diff", "picture", "hook-v2"]);
    assert_eq!(diff_paths(&hook_diff), vec!["slots.hook.binding"]);
    let vo_child: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("scions/dub-en.json")).unwrap()).unwrap();
    assert_eq!(vo_child["parent"], "picture");
    assert_ne!(vo_child["id"], "hook-v2");
    let disjoint = run_ok(&dir, &["diff", "dub-en", "hook-v2"]);
    let mut disjoint_paths = diff_paths(&disjoint);
    disjoint_paths.sort();
    assert_eq!(
        disjoint_paths,
        vec!["slots.hook.binding", "slots.vo.binding"]
    );

    run_ok(&dir, &["scion", "fork", "picture", "dub-hi"]);
    run_ok(
        &dir,
        &[
            "bind",
            "vo",
            vo_en.to_str().unwrap(),
            "--in",
            "0",
            "--out",
            "10",
            "--scion",
            "dub-hi",
            "--layer",
            "copy",
        ],
    );
    let hi = run_ok(
        &dir,
        &["compile", "--scion", "dub-hi", "--out", "dub-hi.mp4"],
    );
    assert_eq!(plan_slot(&hi, "body")["cache"], "hit");
    assert_eq!(plan_slot(&hi, "body")["blob"], body_blob);
    assert_eq!(plan_slot(&hi, "hook")["cache"], "hit");
    assert_eq!(plan_slot(&hi, "cta")["cache"], "hit");
    assert_eq!(plan_overlay_audio(&hi, "vo")["cache"], "hit");
    assert_eq!(
        plan_overlay_audio(&hi, "vo")["slot_encode"],
        vo_audio["slot_encode"]
    );
    assert_ne!(
        plan_overlay_audio(&hi, "vo")["slot_encode"],
        vo_audio2["slot_encode"]
    );
    let hi_diff = run_ok(&dir, &["diff", "picture", "dub-hi"]);
    assert_eq!(diff_paths(&hi_diff), vec!["slots.vo.binding"]);
    let en_hi = run_ok(&dir, &["diff", "dub-en", "dub-hi"]);
    assert_eq!(diff_paths(&en_hi), vec!["slots.vo.binding"]);
    let merged = run_ok(
        &dir,
        &[
            "merge",
            "picture",
            "hook-v2",
            "dub-hi",
            "--id",
            "hook-and-dub",
        ],
    );
    assert!(merged["merged"].is_object());
    assert_eq!(merged["conflicts"], serde_json::json!([]));
    let combined = run_ok(
        &dir,
        &[
            "compile",
            "--scion",
            "hook-and-dub",
            "--out",
            "combined.mp4",
        ],
    );
    assert_eq!(plan_slot(&combined, "body")["cache"], "hit");
    assert_eq!(plan_slot(&combined, "body")["blob"], body_blob);
    assert_eq!(plan_slot(&combined, "hook")["cache"], "hit");
    assert_eq!(plan_slot(&combined, "cta")["cache"], "hit");
    assert_eq!(plan_overlay_audio(&combined, "vo")["cache"], "hit");
    let vs_hook = run_ok(&dir, &["diff", "hook-v2", "hook-and-dub"]);
    assert_eq!(diff_paths(&vs_hook), vec!["slots.vo.binding"]);
    let vs_hi = run_ok(&dir, &["diff", "dub-hi", "hook-and-dub"]);
    assert_eq!(diff_paths(&vs_hi), vec!["slots.hook.binding"]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn ship_fails_without_named_takes() {
    let dir = tmp("ship-empty");
    fs::create_dir_all(dir.join("takes")).unwrap();
    let err = run_fail(&dir, &["ship"]);
    assert!(
        err.contains("no named takes") || err.contains("not a take directory"),
        "{err}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn address_primes_swap_on_worked_example() {
    let dir = copy_example();
    let build = seed_example_build(&dir);
    let got = run_ok(&dir, &["address", "--kind", "hook_rate", "--build", &build]);
    assert_eq!(got["dirty_slots"], serde_json::json!(["hook"]));
    assert_eq!(got["creative_replacement"], "required");
    assert_eq!(got["next"], "graft swap hook <file>");
    assert_eq!(got["scion"], "hook-next");
    assert!(dir.join("scions/hook-next.json").is_file());
    let child: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("scions/hook-next.json")).unwrap())
            .unwrap();
    assert_eq!(child["parent"], "hook_v3+body_v1+cta_v1@9x16");
    assert!(child["layers"]
        .as_array()
        .map(|layers| layers.is_empty())
        .unwrap_or(false));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn swap_hook_prints_reuse_ledger() {
    if !ffmpeg_available() {
        eprintln!("skip swap_hook_prints_reuse_ledger — install ffmpeg");
        return;
    }
    let dir = tmp("porcelain-loop");
    git_init_local(&dir);
    fs::create_dir_all(dir.join("takes/hooks")).unwrap();
    write_color(&dir.join("takes"), "hook.mp4", "red", "1", "30");
    write_color(&dir.join("takes"), "body.mp4", "green", "2", "30");
    write_color(&dir.join("takes"), "cta.mp4", "yellow", "1", "30");
    write_sine(&dir.join("takes"), "vo.wav", "4");
    write_color(&dir.join("takes/hooks"), "v2.mp4", "blue", "1", "30");
    write_color(&dir.join("takes/hooks"), "v3.mp4", "black", "1", "30");
    write_color(&dir.join("takes/hooks"), "v4.mp4", "white", "1", "30");
    fs::write(dir.join("takes/foo.mp4"), b"not-a-role").unwrap();
    fs::write(dir.join("takes/notes.txt"), b"ignore").unwrap();

    let (shipped, ship_err) = run_ok_full(&dir, &["ship", "--out", "ad.mp4"]);
    assert_eq!(shipped["plan"]["encode"], true);
    assert!(dir.join("ad.mp4").is_file());
    assert_eq!(shipped["ledger"]["encoded"], true);
    assert!(
        ship_err.contains("skip unknown take") && ship_err.contains("foo.mp4"),
        "{ship_err}"
    );
    assert!(ship_err.contains("dirty"), "{ship_err}");
    let warmed = run_ok(&dir, &["compile", "--out", "ad-warm.mp4"]);
    assert_eq!(plan_slot(&warmed, "body")["cache"], "hit");
    assert_eq!(plan_slot(&warmed, "cta")["cache"], "hit");
    let body_blob = plan_slot(&warmed, "body")["blob"].clone();
    let cta_blob = plan_slot(&warmed, "cta")["blob"].clone();
    assert!(body_blob.is_string(), "{body_blob}");
    let build = shipped["build"]["id"].as_str().unwrap().to_string();
    let parent = shipped["ledger"]["scion"].as_str().unwrap().to_string();
    assert!(
        shipped["plan"]["overlay_audio"]
            .as_array()
            .is_some_and(|slots| slots.iter().any(|slot| slot["id"] == "vo")),
        "ship should bind takes/vo.wav: {}",
        shipped["plan"]
    );

    git_add_commit(&dir, "ship recipes");
    let tracked = git_tracked_names(&dir);
    assert!(tracked.iter().any(|f| f == "score.json"), "{tracked:?}");
    assert!(
        tracked.iter().any(|f| f.starts_with("scions/")),
        "{tracked:?}"
    );
    assert!(
        !tracked.iter().any(|f| f.starts_with(".graft")),
        "{tracked:?}"
    );
    assert!(!tracked.iter().any(|f| f.ends_with(".mp4")), "{tracked:?}");

    let (swapped, swap_err) = run_ok_full(
        &dir,
        &["swap", "hook", "takes/hooks/v2.mp4", "--out", "ad-v2.mp4"],
    );
    assert!(dir.join("ad-v2.mp4").is_file());
    assert_eq!(plan_slot(&swapped, "hook")["cache"], "miss");
    assert_eq!(plan_slot(&swapped, "body")["cache"], "hit");
    assert_eq!(plan_slot(&swapped, "cta")["cache"], "hit");
    assert_eq!(plan_slot(&swapped, "body")["blob"], body_blob);
    assert_eq!(plan_slot(&swapped, "cta")["blob"], cta_blob);
    let clean_ids: Vec<&str> = swapped["ledger"]["clean"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|slot| slot["id"].as_str())
        .collect();
    assert!(clean_ids.contains(&"body"), "{clean_ids:?}");
    assert!(clean_ids.contains(&"cta"), "{clean_ids:?}");
    assert!(!swapped["ledger"]["dirty"]
        .as_array()
        .unwrap()
        .iter()
        .any(|slot| slot == "body"));
    assert!(swap_err.contains("clean  body"), "{swap_err}");
    assert!(swap_err.contains("dirty  hook"), "{swap_err}");
    let body_in_v1 = copy_video_range(&dir, "ad.mp4", "1", "2", "body-v1.h264");
    let body_in_v2 = copy_video_range(&dir, "ad-v2.mp4", "1", "2", "body-v2.h264");
    assert!(
        !body_in_v1.is_empty() && body_in_v1 == body_in_v2,
        "body GOP inside the linked dest must match across a hook swap"
    );
    assert_ne!(
        fs::read(dir.join("ad.mp4")).unwrap(),
        fs::read(dir.join("ad-v2.mp4")).unwrap()
    );

    let addressed = run_ok(&dir, &["address", "--kind", "hook_rate", "--build", &build]);
    assert_eq!(addressed["next"], "graft swap hook <file>");
    assert_eq!(addressed["creative_replacement"], "required");
    let primed = addressed["scion"].as_str().unwrap().to_string();
    let child = load_scion_json(&dir, &primed);
    assert!(child["layers"]
        .as_array()
        .map(|layers| layers.is_empty())
        .unwrap_or(false));
    let after_address = run_ok(
        &dir,
        &[
            "swap",
            "hook",
            "takes/hooks/v3.mp4",
            "--scion",
            &primed,
            "--out",
            "ad-v3.mp4",
        ],
    );
    assert!(dir.join("ad-v3.mp4").is_file());
    assert_eq!(plan_slot(&after_address, "body")["cache"], "hit");
    assert_eq!(plan_slot(&after_address, "body")["blob"], body_blob);

    git_add_commit(&dir, "address hook-next");
    let history_names = git_history_names(&dir);
    assert!(
        history_names.iter().any(|f| f.starts_with("feedback/")),
        "{history_names:?}"
    );
    assert!(
        history_names
            .iter()
            .any(|f| f == &format!("scions/{primed}.json")),
        "{history_names:?}"
    );
    assert!(
        !history_names
            .iter()
            .any(|f| f.starts_with(".graft") || f.ends_with(".mp4")),
        "{history_names:?}"
    );

    let batch = run_ok(
        &dir,
        &[
            "swap",
            "hook",
            "--from",
            "takes/hooks",
            "--out-dir",
            "out",
            "--scion",
            &parent,
        ],
    );
    assert_eq!(batch["variants"].as_array().unwrap().len(), 3);
    for variant in batch["variants"].as_array().unwrap() {
        assert_eq!(plan_slot(variant, "body")["cache"], "hit");
        assert_eq!(plan_slot(variant, "body")["blob"], body_blob);
    }

    fs::remove_dir_all(dir.join(".graft/actions")).unwrap();
    let err = run_fail(
        &dir,
        &[
            "swap",
            "hook",
            "takes/hooks/v4.mp4",
            "--scion",
            &parent,
            "--out",
            "ad-fail.mp4",
        ],
    );
    assert!(
        err.contains("recoded clean siblings") && err.contains("principle 14"),
        "{err}"
    );
    let _ = fs::remove_dir_all(&dir);
}
