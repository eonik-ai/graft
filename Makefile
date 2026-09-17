# SPDX-License-Identifier: Apache-2.0
.PHONY: test lint fmt fmt-check readme-demo demo walkthrough example-encode clean

PYTHON ?= python3
CARGO ?= cargo
export PYTHONPATH := ref

test:
	$(PYTHON) scripts/validate.py
	$(PYTHON) -m unittest discover -s ref/tests -v
	$(CARGO) test --workspace --locked

lint:
	$(PYTHON) -m compileall -q ref scripts
	$(PYTHON) scripts/validate.py
	$(CARGO) fmt --all -- --check
	$(CARGO) clippy --workspace --all-targets --locked -- -D warnings

fmt:
	$(CARGO) fmt --all

fmt-check: lint

readme-demo:
	$(CARGO) build --release --bin graft
	sh scripts/record-landing.sh

demo:
	$(CARGO) build --release --bin graft
	sh scripts/demo.sh

walkthrough:
	$(CARGO) build --release --bin graft
	sh scripts/walkthrough.sh

example-encode:
	$(CARGO) build --release --bin graft
	sh scripts/example-encode.sh

clean:
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
	$(CARGO) clean
