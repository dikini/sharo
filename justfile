set shell := ["bash", "-euo", "pipefail", "-c"]

setup:
    scripts/bootstrap-dev.sh --apply

init-repo:
    scripts/init-repo.sh --apply

extract-backbone:
    scripts/extract-backbone.sh

init-backbone-repo dest project='':
    if [ -n "{{project}}" ]; then scripts/init-from-backbone.sh --dest "{{dest}}" --project "{{project}}"; else scripts/init-from-backbone.sh --dest "{{dest}}"; fi

verify:
    scripts/check-fast-feedback.sh

fast-feedback:
    scripts/check-fast-feedback.sh

merge-gate:
    scripts/check-merge-result.sh

daemon-invariants:
    scripts/check-daemon-invariants.sh

shell-quality:
    scripts/check-shell-quality.sh --all

workflow-lint:
    scripts/check-workflows.sh

rust-hygiene:
    scripts/check-rust-hygiene.sh --advisory --check all

openai-live-smoke:
    scripts/openai-live-smoke.sh
