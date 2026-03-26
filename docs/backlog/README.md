<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# CNC Control V2 Backlog Workspace

`docs/backlog/` je kanonski dom za implementacioni backlog
`lionsos-cnc-control-v2` linije.

Poenta ovog foldera nije da zameni arhitektonske dokumente iz `docs/`, nego da
iz njih izvuce usko definisane, reviewable backlog rezove.

## Source Of Truth

Arhitektonski source of truth ostaje:

- `README.md`
- `docs/ARCHITECTURE.md`
- `docs/TOPOLOGY.md`
- `docs/MCU_BOUNDARY.md`
- `docs/CHANNEL_REGION_MODEL.md`
- `docs/SUBSYSTEM_MCU_TRANSPORT.md`
- `docs/SUBSYSTEM_MCU_EXECUTOR.md`
- `include/cnc_v2/*.h`
- `meta.py`
- `lionsos_cnc_v2.mk`
- `Makefile`

Backlog dokumenti ne smeju tiho menjati te boundary-je. Ako otkriju rupu ili
kontradikciju, moraju je izneti kao work item ili human-owned decision.

## Canonical Documents

- [B00 Implementation Master](B00_IMPLEMENTATION_MASTER.md)
- [B10 Runtime Wiring And Contract Freeze](B10_RUNTIME_WIRING_AND_CONTRACT_FREEZE.md)
- [B20 Minimal Bring-Up And Smoke Acceptance](B20_MINIMAL_BRINGUP_AND_SMOKE_ACCEPTANCE.md)
- [B30 MCU Transport And Executor Hardening](B30_MCU_TRANSPORT_AND_EXECUTOR_HARDENING.md)
- [Backlog Template](BACKLOG_TEMPLATE.md)

## Naming Rules

- veliki backlog dokumenti koriste prefiks `Bxx_`
- buduci manji AI-ready slice dokumenti koriste `Sxx_` ili `Bxx-Syy_`
- work item identifikatori unutar backloga koriste format `B10-001`,
  `B20-004`, `B30-007`

## Status Vocabulary

Koristi samo sledece statuse:

- `planned`
- `active`
- `blocked`
- `partial`
- `done`

## Extraction Rule

- implementacija ne treba da krece direktno iz master backloga
- svaki stvarni coding korak treba da se izvuce iz jednog child backloga kao
  mali AI-ready slice
- slice mora da ima goal, in scope, out of scope, stable boundaries,
  dependencies, invariants, deliverables i acceptance

## Current Active Tracks

Trenutni backlog program je namerno uzak:

- `B10` zatvara i kanonizuje vec postojeci contract/wiring runtime cut
- `B20` zakljucava prvi kanonski bring-up i smoke acceptance put
- `B30` hardenuje Pi/MCU wire semantics bez sirenja authority graph-a

Redosled je kanonski:

1. `B10`
2. `B20`
3. `B30`
