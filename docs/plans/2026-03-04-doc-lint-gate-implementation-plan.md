# Doc Lint Gate Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a lightweight, minimal-dependency documentation lint gate that reduces uncertainty and catches high-value doc regressions.

**Architecture:** Implement a single shell script (`scripts/doc-lint.sh`) using `bash`, `find`, `rg`, and standard POSIX tooling. Encode lint policy in a top-of-file block comment so future edits preserve rule discipline.

**Tech Stack:** Bash, ripgrep, find, sed, git.

---

### Task 1: Add policy and script scaffold

**Files:**
- Create: `/home/dikini/Projects/sharo/scripts/doc-lint.sh`

**Step 1: Add policy block comment at top**

Include:
- evergreen rules vs temporary regression guards
- temporary-guard metadata convention (`id`, `reason`, `added`, `review_by`)
- expectation that temporary rules are pruned

**Step 2: Add script strict mode and helpers**

Use `set -euo pipefail`, `fail()` helper, and clear final pass/fail reporting.

### Task 2: Implement initial lightweight checks

**Files:**
- Modify: `/home/dikini/Projects/sharo/scripts/doc-lint.sh`

**Step 1: Scope**

Lint only canonical repo docs:
- `docs/**/*.md`
- `AGENTS.md`

**Step 2: Broken markdown link check**

Validate markdown links that point to local files.
Skip:
- `http://`, `https://`, `mailto:`
- anchor-only links (`#...`)

**Step 3: Temporary known-regression guard**

Add one temporary guard for stale spec path:
- reject the legacy MVP spec path and require `docs/specs/mvp.md`

Include temporary-guard metadata comment.

### Task 3: Verify and operationalize

**Files:**
- Modify: `/home/dikini/Projects/sharo/scripts/doc-lint.sh`

**Step 1: Make script executable**

Run: `chmod +x /home/dikini/Projects/sharo/scripts/doc-lint.sh`

**Step 2: Run lint**

Run: `/home/dikini/Projects/sharo/scripts/doc-lint.sh`
Expected: `doc-lint: OK` or actionable failures.

**Step 3: Confirm script is discoverable**

Run: `ls -la /home/dikini/Projects/sharo/scripts/doc-lint.sh`
