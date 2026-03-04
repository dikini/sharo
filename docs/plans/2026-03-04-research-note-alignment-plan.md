# Research Note Alignment Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Update the main Sharo research note so it reflects the currently integrated design conclusions while making it explicit where the research frontier is still ahead of the current architecture and MVP plan.

**Architecture:** Edit only `/home/dikini/Knot/Starter/sharo/research/agent-research.md`. Keep the addenda independent as source-specific notes, link to them as references, and mark in the main research note which conclusions are already integrated versus which remain ahead of the present design and planning baseline.

**Tech Stack:** Markdown notes in Knot vault, local planning docs, `rg`, `sed`, and `apply_patch`.

---

### Task 1: Identify the integration points

**Files:**
- Modify: `/home/dikini/Knot/Starter/sharo/research/agent-research.md`
- Reference: `/home/dikini/Projects/sharo/docs/specs/mvp.md`

**Step 1: Locate the sections where research conclusions drive design**

Run: `rg -n "Executive Summary|Memory Should Be Tiered|Memory, Learning, and Self-Improvement|Minimal Design for Your Tool|Incremental Implementation Strategy|Starter Summary" /home/dikini/Knot/Starter/sharo/research/agent-research.md`
Expected: The main synthesis sections are identified.

**Step 2: Preserve addenda independence**

Do not edit the addenda. Only reference them from the main research note.

### Task 2: Add explicit integration status

**Files:**
- Modify: `/home/dikini/Knot/Starter/sharo/research/agent-research.md`

**Step 1: Add a short framing note near the top**

State that some research conclusions are already integrated into design and planning, while others are intentionally ahead of the current MVP.

**Step 2: Mark the memory line clearly**

Show that bounded active memory, exact/derived distinctions, salience metadata, and supersession hooks are integrated, while stronger memory controllers, richer consolidation jobs, and more advanced forgetting remain ahead.

**Step 3: Mark the harness line clearly**

Show that trace-first verification, agent-legible notes/plans, and minimal harness hooks are integrated, while richer observability-facing capabilities, recurring cleanup loops, and stronger machine-checkable architecture enforcement remain ahead.

### Task 3: Verify the note

**Files:**
- Verify: `/home/dikini/Knot/Starter/sharo/research/agent-research.md`

**Step 1: Run targeted consistency checks**

Run: `rg -n "Integrated into current design|Ahead of current design|openai-harness-engineering-codex|google-memory-papers-titans-miras|observability|cleanup|salience|supersession" /home/dikini/Knot/Starter/sharo/research/agent-research.md`
Expected: The research note clearly distinguishes integrated conclusions from research-ahead items and still links to the addenda.

**Step 2: Re-read the edited sections**

Confirm the addenda remain independent and the main note accurately describes the current design/planning state without overclaiming implementation.
