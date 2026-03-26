<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# CNC Control V2 Implementation Master Backlog

## Svrha

Ovaj dokument je glavni backlog entrypoint za prvi stvarni implementacioni
program u `lionsos-cnc-control-v2`.

Njegova uloga nije da zamenjuje arhitekturu, nego da:

- poveze postojeci runtime cut sa implementacionim redosledom
- izvuce kanonske backlog pravce
- zakljuca zavisnosti i redosled
- spreci da rad krene iz kontradiktornih runtime ili acceptance pretpostavki

## Status

Ovaj backlog je trenutno **active**.

## Reality Check

Danasnje stanje repoa je jace od "appliance scaffold" opisa iz kratkog
`README.md`:

- Pi-side app graph je vec imenovan, izgenerisan i buildable
- `meta.py` + `lionsos_cnc_v2.mk` vec pokrivaju dve board putanje:
  `qemu_virt_aarch64` i `rpi3b`
- `include/cnc_v2/*.h` vec zakljucava channels, regions, config blob layouts,
  generation helpers i wire ABI
- `components/` vec nosi uski `job -> plan -> safety -> transport ->
  observability` tok
- `mcu/` vec implementira sinteticki bare-metal executor sa `ACK/NAK/status` i
  heartbeat odgovorima

I dalje nedostaje:

- jedan prvi zamrznuti smoke acceptance put iznad build contract-a
- hardenovan transport/executor failure model, posebno oko timeout/retry i
  response klasifikacije

Backlog workspace je sada uveden da upravo te preostale korake drzi
kanonski vezanim za stvarno stanje repoa.

## Source Of Truth

Arhitektonski input za ovaj backlog ostaje:

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

## Track Snapshot

- [B10 Runtime Wiring And Contract Freeze](B10_RUNTIME_WIRING_AND_CONTRACT_FREEZE.md) -
  `done`
- [B20 Minimal Bring-Up And Smoke Acceptance](B20_MINIMAL_BRINGUP_AND_SMOKE_ACCEPTANCE.md) -
  `active`
- [B30 MCU Transport And Executor Hardening](B30_MCU_TRANSPORT_AND_EXECUTOR_HARDENING.md) -
  `planned`

## Canonical Sequence

Redosled backlog zatvaranja je kanonski:

1. `B10` prvo zatvara sta je vec stvarno freeze-ovano u docs, contracts,
   `meta.py`, `.mk` i build izlazima
2. `B20` zatim zakljucava prvi bring-up target, build path i minimalni smoke
   acceptance scenario
3. `B30` tek onda hardenuje wire-level i MCU executor semantics bez pomeranja
   authority ili topology granica

## Cross-Track Invariants

Sva tri pravca moraju istovremeno da cuvaju sledece:

- Pi-side control plane ostaje `seL4 + Microkit + sDDF + LionsOS`
- MCU ostaje bare-metal executor, ne "drugi seL4 cvor"
- `mcu_transport` ostaje jedini app PD sa serial-client ownership-om
- `state_core` ostaje authoritative owner lifecycle-a i kanonskog
  machine-state modela na Pi strani
- forbidden shortcuts iz `docs/TOPOLOGY.md` ostaju zabranjeni
- nema app-side direktnog UART MMIO ownership-a
- session persistence i siri host UX ne smeju se uvoditi kroz transport backlog

## Blocker Table

Trenutni blokeri po pravcu:

- `B10`
  - nema aktivnih contract/topology blokera
  - backlog ostaje kao kanonski close-out vec postojecih odluka
- `B20`
  - prvi kanonski bring-up target i build contract su sada freeze-ovani
  - `README.md` jos ne nosi puni smoke status repoa
  - nema jednog backlog-izvucenog smoke scenarija sa eksplicitnim success
    signalima po subsystem-u
- `B30`
  - nema retransmit ili bounded retry policy-ja
  - validan RX frame danas automatski zavrsava wait stanje i postavlja
    `RESPONSE_OK` bez pune response klasifikacije
  - heartbeat i status/position odgovori jos nisu backlog-locked kao odvojene
    semanticke klase

## Exit Criteria For This Master Backlog

Master backlog je zatvoren kada su istovremeno tacne sledece stvari:

- `B10`, `B20` i `B30` imaju `done` status
- postoji jedan kanonski build i smoke path za prvi runtime cut
- transport failure semantics su eksplicitne i reviewable
- sledeci rad moze da se deli na manje feature backlogove ili AI-ready
  slice-eve bez povratka na osnovni runtime/acceptance spor

## Slice Extraction Rule

Iz ovog dokumenta se ne implementira direktno.

Pre svakog coding koraka potrebno je:

1. izabrati jedan child backlog
2. izvuci mali slice brief
3. zakljucati goal, in scope, out of scope, stable boundaries i acceptance
4. tek onda dati AI-ju implementaciju

## First Expected Slice Families

Prve logicne slice porodice koje ce izaci iz ovog master backloga su:

- `B20` slice za smoke acceptance evidenciju
- `B20` slice za subsystem success signal model i README alignment
- `B30` slice za response-class model
- `B30` slice za timeout/retry policy
- `B30` slice za state_core/observability alignment posle transport hardening-a
