<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# CNC Control V2 Backlog Template

Use this template for every new backlog document in `docs/backlog/`.

The purpose of the template is to keep the backlog precise enough for AI
implementation and human review, without any vague "just make it work"
space.

## When To Use It

Use it for:

- a new larger backlog direction
- a new backlog topic that has multiple work items
- a backlog that needs to be cut into multiple AI-ready slices

Do not use it for:

- a larger architectural manifest or README
- a one-off decision without backlog elaboration
- one small coding task that already has a clear slice brief

## Required Sections

Every backlog document must include the following sections:

- `Purpose`
- `Status`
- `Reality Check`
- `Definition Of Done`
- `Canonical Artifacts`
- `Dependencies`
- `Work Items`
- `Out Of Scope`
- `Human-Owned Decisions`

Optional sections:

- `Chosen Defaults`
- `Acceptance Sequence`
- `Risks`
- `Future Slices`

## Canonical Skeleton

```md
# <short backlog name>

## Purpose

- what the backlog covers
- where its boundary is

## Status

This backlog is currently **planned|active|blocked|partial|done**.

## Reality Check

- what exists today
- what is missing
- which contradictions or gaps exist

## Definition Of Done

- which conditions must all be true at the same time
- how we know the backlog is actually closed

## Canonical Artifacts

- which files, headers, generated artifacts, or docs are canonical

## Dependencies

- which other backlog documents or docs must already be frozen

## Work Items

### Bxx-001 <short name>

Task:

- what needs to be frozen or delivered

Acceptance:

- how we know this item is done

Status:

- planned|active|blocked|partial|done

## Out Of Scope

- what the backlog intentionally does not cover

## Human-Owned Decisions

- what the AI must not decide on its own
- which decisions remain human-owned
```

## Rules For Good Backlogs

- the goal must be described as an outcome, not as a mechanical list of
  commands
- `reality check` must describe the actual current state, not just the ideal
  target
- work items must have stable ids
- every work item must have acceptance and status
- the backlog must have an explicit `out of scope`
- if there is a high-impact decision, it must appear in
  `Human-Owned Decisions`, not be hidden in the prose

## Slice Extraction Rules

When a backlog is ready for implementation, extract a separate slice brief from
it.

Each slice must freeze:

- one main goal
- a small number of files or one clear subsystem
- stable boundaries it must not move
- acceptance that can be verified

If one slice naturally requires multiple independent acceptance steps, it
should be split into multiple slices.
