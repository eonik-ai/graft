# SPDX-License-Identifier: Apache-2.0
.PHONY: test lint fmt clean

PYTHON ?= python3
export PYTHONPATH := ref

test:
	$(PYTHON) scripts/validate.py
	$(PYTHON) -m unittest discover -s ref/tests -v

lint:
	$(PYTHON) -m compileall -q ref scripts
	$(PYTHON) scripts/validate.py

fmt:
	@echo "no formatter pinned yet"

fmt-check: lint

clean:
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
