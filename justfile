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

verify-ci:
    scripts/check-ci-smoke.sh

fast-feedback:
    scripts/check-fast-feedback.sh

prepush-policy:
    scripts/check-prepush-policy.sh

merge-gate:
    scripts/check-merge-result.sh

daemon-invariants:
    scripts/check-daemon-invariants.sh

flaky-regressions:
    scripts/check-flaky-regressions.sh --changed

shell-quality:
    scripts/check-shell-quality.sh --all

doc-portability:
    scripts/check-doc-portability.sh --all

workflow-lint:
    scripts/check-workflows.sh

rust-hygiene:
    scripts/check-rust-hygiene.sh --advisory --check all

openai-live-smoke:
    scripts/openai-live-smoke.sh

docker-build:
    scripts/docker-build.sh

docker-smoke:
    scripts/docker-smoke.sh
