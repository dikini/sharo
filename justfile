set shell := ["bash", "-euo", "pipefail", "-c"]

setup:
    scripts/bootstrap-dev.sh --apply

verify:
    scripts/check-fast-feedback.sh

fast-feedback:
    scripts/check-fast-feedback.sh

merge-gate:
    scripts/check-merge-result.sh

daemon-invariants:
    scripts/check-daemon-invariants.sh
