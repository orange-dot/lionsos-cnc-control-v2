<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# B20 Minimal Bring-Up And Smoke Acceptance

## Svrha

Ovaj backlog zakljucava prvi kanonski bring-up i smoke acceptance put za
`lionsos-cnc-control-v2`.

Granica backloga je namerno uska:

- cilj je jedan reviewable build i acceptance red
- cilj nije jos raditi transport retransmit, persistence ili siri host tooling

## Status

Ovaj backlog je trenutno **active**.

## Reality Check

Danas vec postoje:

- `Makefile` kao top-level build entrypoint
- `lionsos_cnc_v2.mk` kao stvarni build/wiring sloj
- `meta.py` kao system description generator
- `build/` i `build-rpi3b/` sa vec generisanim image/report artefaktima
- prvi kanonski build contract sada je freeze-ovan oko
  `qemu_virt_aarch64/debug` sa `BUILD_DIR=build`
- `rpi3b` ostaje parity build target sa `BUILD_DIR=build-rpi3b`
- realan app-side command path kroz `job_ingress`, `state_core`, `planner`,
  `safety_coordinator`, `mcu_transport`, `session_store`, `observability`

Jos uvek nedostaje:

- jedan backlog-zakljucan smoke scenario sa named success signalima
- potpuno zatvoren subsystem-level success signal model
- stvarni runtime smoke close-out iznad build-plus-artifact sloja

## Chosen Defaults

Za prvi acceptance cut vaze sledeci default-i:

- prvi kanonski bring-up target je `qemu_virt_aarch64` sa `MICROKIT_CONFIG=debug`
- `rpi3b` ostaje parity build target, ali ne i prvi obavezni run target
- build-plus-generated-artifact evidence je dovoljan da zatvori `B20-001`
- stvarni `qemu` boot/log evidence je pomeren u `B20-002`
- prvi smoke scenario pokriva jedan accepted job koji prolazi kroz:
  `state_core -> planner -> safety_coordinator -> mcu_transport ->
  state_core -> observability`

## Definition Of Done

Ovaj backlog je gotov kada su istovremeno tacne sledece stvari:

- postoji jedan kanonski build path za prvi bring-up target
- postoji jedan minimalni smoke scenario sa jasno imenovanim success signalima
- success signali pokrivaju build artifact, topology artifact i runtime-facing
  state/transport/observability efekte
- `README.md` i backlog vise ne daju razlicitu sliku o tome sta je "trenutni
  runtime cut"

## Canonical Artifacts

- `README.md`
- `Makefile`
- `lionsos_cnc_v2.mk`
- `meta.py`
- `build/report.txt`
- `components/state_core/state_core.c`
- `components/planner/planner.c`
- `components/safety_coordinator/safety_coordinator.c`
- `components/mcu_transport/mcu_transport.c`
- `components/observability/observability.c`

## Dependencies

- `B10` mora ostati zatvoren i neotvoren

## Acceptance Sequence

Prvi acceptance red za ovaj backlog treba da zakljuca:

1. koji je kanonski build command path za prvi bring-up target
2. koji generated artefakti moraju postojati posle build-a
3. koji minimalni job scenario pokrecemo ili simuliramo
4. koje promene u `state_snapshot`, `transport_status` i
   `observability_report` predstavljaju uspeh
5. koji failure signali odmah znace da bring-up cut nije zatvoren

## Work Items

### B20-001 Freeze first bring-up target and build contract

Zadatak:

- zamrznuti `qemu_virt_aarch64` sa `MICROKIT_CONFIG=debug` kao prvi kanonski
  bring-up target
- zakljucati top-level build command:
  `LIONSOS=/path/to/lionsos MICROKIT_SDK=/path/to/microkit-sdk make`
- zakljucati parity build command za `rpi3b`:
  `LIONSOS=/path/to/lionsos MICROKIT_SDK=/path/to/microkit-sdk MICROKIT_BOARD=rpi3b BUILD_DIR=build-rpi3b make`
- zakljucati checklist primary output artefakata za prvi target:
  `build/lionsos_cnc_v2.img`, `build/lionsos_cnc_v2.system`,
  `build/report.txt`, `build/qemu_virt_aarch64.dtb`, app-side `.elf` image-i
  i generated `.data` wiring blobovi

Acceptance:

- novi implementer ne mora da pogadja da li prvo cilja `qemu_virt_aarch64` ili
  `rpi3b`, niti koji output znaci da je build uspeo
- `README.md` i backlog koriste isti command contract i isti artifact checklist

Status:

- done

### B20-002 Publish minimal smoke scenario

Zadatak:

- opisati jedan uski smoke scenario koji dokazuje da command path nije samo
  compile-only
- scenario mora da obuhvati accepted job i downstream state/transport efekte

Acceptance:

- postoji jedan reviewable scenario koji ne siri scope na pune CNC funkcije

Status:

- planned

### B20-003 Freeze subsystem-level success signals

Zadatak:

- imenovati sta se racuna kao success signal za:
  - `state_core`
  - `mcu_transport`
  - `observability`
- jasno odvojiti build artifact success od runtime success-a

Acceptance:

- bring-up review vise ne zavisi od neizrecenog "deluje da je dovoljno dobro"

Status:

- planned

### B20-004 Align README with canonical runtime cut

Zadatak:

- dopuniti repo-level opis tako da dokumentuje backlog workspace, current
  status i prvi acceptance put

Acceptance:

- `README.md` vise ne zaostaje za backlog-locked runtime stvarnoscu

Status:

- partial

## Out Of Scope

- transport retransmit ili advanced recovery politika
- session persistence iza in-memory stuba
- novi job tipovi, planner feature-i ili host-side tooling
- uvodjenje dodatnih PD-ova ili promena authority graph-a

## Human-Owned Decisions

- da li `B20-002` zatvaramo preko stvarnog `qemu` boot/log-a, ili preko drugog
  reviewable smoke evidence sloja
- da li `rpi3b` runtime smoke mora biti deo istog backlog close-outa ili
  zaseban follow-up cut
