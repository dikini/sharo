# Design Note Alignment Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Align every note under `sharo/design/` with the revised MVP spec so coordination, memory durability, and minimal harness hooks are described consistently across the design set.

**Architecture:** Treat [mvp.md](/home/dikini/Projects/sharo/docs/specs/mvp.md) as the canonical contract and apply the same structural deltas across the subsystem notes without over-expanding MVP scope. Keep coordination operationally minimal but durable and visible, keep memory exact-record semantics consistent, and add early observability / verification / cleanup hooks where each subsystem naturally touches them.

**Tech Stack:** Markdown notes in Knot vault, local spec document in repo, `rg`, `sed`, `apply_patch`, and targeted verification commands.

---

### Task 1: Capture the alignment deltas

**Files:**
- Modify: `/home/dikini/Projects/sharo/docs/specs/mvp.md`
- Modify: `/home/dikini/Knot/Starter/sharo/design/*.md`

**Step 1: Re-read the canonical MVP deltas**

Run: `rg -n "multiple concurrent sessions|coordination|PolicyDecision|observability|cleanup|salience|supersession|Scenario C" /home/dikini/Projects/sharo/docs/specs/mvp.md`
Expected: The revised MVP sections for coordination, memory durability, and harness hooks are visible.

**Step 2: Group design notes by edit type**

Group A: hub and protocol notes.
Group B: execution and control notes.
Group C: memory, policy, and security notes.

**Step 3: Keep edits YAGNI**

Do not add rich arbitration, new transports, or heavyweight observability systems. Add only structural language and invariants needed to avoid future drift.

### Task 2: Align hub and protocol notes

**Files:**
- Modify: `/home/dikini/Knot/Starter/sharo/design/minimal-design.md`
- Modify: `/home/dikini/Knot/Starter/sharo/design/daemon-protocol.md`
- Modify: `/home/dikini/Knot/Starter/sharo/design/operator-surfaces.md`

**Step 1: Update minimal architecture framing**

Add explicit mention that:
- concurrent sessions are in-scope for MVP
- coordination is structurally present but not richly arbitrated
- minimal harness hooks for verification, observability, and cleanup exist from day one

**Step 2: Update protocol framing**

Make protocol visibility for coordination explicit without requiring dedicated coordination-heavy CLI commands.

**Step 3: Update surface framing**

Make CLI-first MVP clear while still preserving trace, artifact, coordination, and harness-hook visibility across future surfaces.

### Task 3: Align execution and control notes

**Files:**
- Modify: `/home/dikini/Knot/Starter/sharo/design/daemon-control-plane.md`
- Modify: `/home/dikini/Knot/Starter/sharo/design/agent-kernel.md`
- Modify: `/home/dikini/Knot/Starter/sharo/design/reasoning-engine.md`
- Modify: `/home/dikini/Knot/Starter/sharo/design/capability-plane.md`
- Modify: `/home/dikini/Knot/Starter/sharo/design/model-routing.md`

**Step 1: Preserve coordination as durable state**

Make sure each note that touches admission, execution, or routing acknowledges coordination context and overlap as durable runtime input or output where relevant.

**Step 2: Add minimal harness hooks**

Where appropriate, mention verification artifacts, observability snapshots, and cleanup or drift markers as valid MVP outputs or extension hooks.

**Step 3: Keep boundaries narrow**

Do not let these notes imply rich automation or arbitration beyond the MVP spec.

### Task 4: Align memory, policy, and security notes

**Files:**
- Modify: `/home/dikini/Knot/Starter/sharo/design/memory-system.md`
- Modify: `/home/dikini/Knot/Starter/sharo/design/personality-and-policy.md`
- Modify: `/home/dikini/Knot/Starter/sharo/design/autonomy-and-approvals.md`
- Modify: `/home/dikini/Knot/Starter/sharo/design/security-model.md`

**Step 1: Normalize exact-record language**

Ensure `PolicyDecision`, `Approval`, and coordination records are consistently treated as exact durable state.

**Step 2: Normalize memory-control language**

Carry through `MemoryRecordClass`, salience metadata, and supersession hooks where relevant without overclaiming automated behavior.

**Step 3: Normalize harness and visibility language**

Ensure verification, observability, and restart-safe evidence are described consistently across these notes.

### Task 5: Verify the note set

**Files:**
- Verify: `/home/dikini/Knot/Starter/sharo/design/*.md`
- Verify: `/home/dikini/Projects/sharo/docs/specs/mvp.md`

**Step 1: Run consistency checks**

Run: `rg -n "multiple concurrent|IntentAnnouncement|ConflictRecord|coordination|PolicyDecision|observability|cleanup|salience|supersession" /home/dikini/Knot/Starter/sharo/design /home/dikini/Projects/sharo/docs/specs/mvp.md`
Expected: Each concept appears where relevant across the design set.

**Step 2: Spot-check note boundaries**

Read representative sections from each updated note and confirm they stay within subsystem scope.

**Step 3: Confirm no obvious drift remains**

Re-review `minimal-design.md`, `memory-system.md`, `daemon-protocol.md`, and `operator-surfaces.md` against the revised MVP plan.
