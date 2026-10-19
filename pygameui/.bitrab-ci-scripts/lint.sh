#!/usr/bin/env bash
set -euo pipefail
source ./.bitrab-ci-scripts/setup.sh
uv run isort --check-only merit_ledger tests
uv run black --check merit_ledger tests
uv run ruff check --quiet merit_ledger tests
uv run pylint --score=n --reports=n --rcfile=.pylintrc merit_ledger
uv run pylint --score=n --reports=n --rcfile=.pylintrc_tests tests
