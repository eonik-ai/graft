#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Load every schema and example JSON. Optionally validate with jsonschema."""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
SCHEMA = REPO / "schema"
EXAMPLES = REPO / "examples"


def load_json(path: Path) -> object:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise SystemExit(f"invalid JSON: {path}: {exc}") from exc


def structural_score(doc: dict, path: Path) -> None:
    for key in ("graft", "concept", "clock", "slots"):
        if key not in doc:
            raise SystemExit(f"{path}: missing {key}")
    if doc["graft"] != "0.2.0":
        raise SystemExit(f"{path}: graft version {doc['graft']!r} != '0.2.0'")
    if not doc["slots"]:
        raise SystemExit(f"{path}: slots must be non-empty")


def structural_scion(doc: dict, path: Path) -> None:
    for key in ("graft", "id", "concept", "dest", "layers"):
        if key not in doc:
            raise SystemExit(f"{path}: missing {key}")
    if "9:16" in doc["id"] and "9x16" not in doc["id"]:
        raise SystemExit(f"{path}: dest axis is 9x16, not a slot named 9:16")


def structural_time_map(doc: dict, path: Path) -> None:
    for key in ("graft", "build", "scion", "scion_hash", "dest_id", "rate", "entries"):
        if key not in doc:
            raise SystemExit(f"{path}: missing {key}")


def maybe_jsonschema(schema_path: Path, instance: object, path: Path) -> None:
    try:
        import jsonschema
    except ImportError:
        return
    schema = load_json(schema_path)
    jsonschema.Draft202012Validator(schema).validate(instance)  # type: ignore[no-untyped-call]


def check_example_consistency(example: Path) -> None:
    score = load_json(example / "score.json")
    scion = load_json(example / "scion.json")
    time_map = load_json(example / "time-map.json")
    if not isinstance(score, dict) or not isinstance(scion, dict) or not isinstance(time_map, dict):
        raise SystemExit(f"{example}: score/scion/time-map must be objects")
    slot_ids = {slot["id"] for slot in score["slots"]}
    bindings = {}
    for layer in scion["layers"]:
        bindings.update(layer["bindings"])
    unknown = set(bindings) - slot_ids
    if unknown:
        raise SystemExit(f"{example}: scion binds unknown slots {sorted(unknown)}")
    required = {slot["id"] for slot in score["slots"] if not slot.get("optional")}
    unbound = required - set(bindings)
    if unbound:
        raise SystemExit(f"{example}: required slots unbound {sorted(unbound)}")
    map_slots = {entry["slot"] for entry in time_map["entries"]}
    if not map_slots <= slot_ids:
        raise SystemExit(
            f"{example}: time-map slots not in score {sorted(map_slots - slot_ids)}"
        )
    if scion["concept"] != score["concept"]:
        raise SystemExit(f"{example}: scion.concept != score.concept")


def main() -> int:
    for schema_file in sorted(SCHEMA.glob("*.schema.json")):
        load_json(schema_file)
        print(f"ok schema {schema_file.relative_to(REPO)}")

    checks = {
        "score.json": (SCHEMA / "score.schema.json", structural_score),
        "scion.json": (SCHEMA / "scion.schema.json", structural_scion),
        "time-map.json": (SCHEMA / "time-map.schema.json", structural_time_map),
    }
    for example in sorted(p for p in EXAMPLES.iterdir() if p.is_dir()):
        for name, (schema_path, struct) in checks.items():
            path = example / name
            if not path.exists():
                raise SystemExit(f"missing {path}")
            doc = load_json(path)
            if not isinstance(doc, dict):
                raise SystemExit(f"{path}: expected object")
            struct(doc, path)
            maybe_jsonschema(schema_path, doc, path)
            print(f"ok example {path.relative_to(REPO)}")
        check_example_consistency(example)
        dirty = example / "dirty.json"
        if dirty.exists():
            load_json(dirty)
            print(f"ok example {dirty.relative_to(REPO)}")
        for extra_dirty in sorted(example.glob("dirty-*.json")):
            load_json(extra_dirty)
            print(f"ok example {extra_dirty.relative_to(REPO)}")
        scions_dir = example / "scions"
        if scions_dir.is_dir():
            for path in sorted(scions_dir.glob("*.json")):
                doc = load_json(path)
                if not isinstance(doc, dict):
                    raise SystemExit(f"{path}: expected object")
                structural_scion(doc, path)
                maybe_jsonschema(SCHEMA / "scion.schema.json", doc, path)
                print(f"ok example {path.relative_to(REPO)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
