<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# B10 Runtime Wiring And Contract Freeze

## Svrha

Ovaj backlog kanonizuje trenutno vec postojeci runtime cut:

- named app graph
- channels i shared regions
- typed contracts
- config blob layouts
- generator i `.mk` wiring

Granica backloga je namerno uska:

- cilj je close-out onoga sto je vec realno freeze-ovano
- cilj nije jos uvoditi nove feature-e, persistence ili transport hardening

## Status

Ovaj backlog je trenutno **done**.

## Reality Check

Danas su sledece stvari vec zakljucene i u runtime smislu konzistentne:

- `docs/TOPOLOGY.md` i `docs/CHANNEL_REGION_MODEL.md` daju isti named edge set
- `include/cnc_v2/control_ipc.h`, `runtime_topology.h` i `config.h` vec
  zakljucavaju payload, channel i config blob identitete
- `meta.py` vec serijalizuje config blobove koji odgovaraju tim layout-ima
- `lionsos_cnc_v2.mk` vec wira iste artefakte u stvarne buildable image-e
- `build/` i `build-rpi3b/` vec pokazuju da prvi runtime cut postoji za oba
  board target-a

`README.md` jos uvek zaostaje za tim stanjem, ali to je doc-gap, ne contract
blokada.

## Definition Of Done

Ovaj backlog je gotov kada su istovremeno tacne sledece stvari:

- svaki named edge ima jasno:
  - jednog producenta
  - jednog potrosaca
  - kanal ili notify smer
  - shared region
  - typed payload contract
- config blob layout u `meta.py` i `include/cnc_v2/config.h` su 1:1
- nijedan app PD ne dobija siri authority nego sto topology tvrdi
- dve board putanje (`qemu_virt_aarch64`, `rpi3b`) koriste isti kanonski
  service graph

## Canonical Artifacts

- `docs/TOPOLOGY.md`
- `docs/CHANNEL_REGION_MODEL.md`
- `include/cnc_v2/config.h`
- `include/cnc_v2/control_ipc.h`
- `include/cnc_v2/runtime_topology.h`
- `include/cnc_v2/wire_protocol.h`
- `meta.py`
- `lionsos_cnc_v2.mk`
- `build/report.txt`
- `build-rpi3b/report.txt`

## Dependencies

- nema prethodnih backlog zavisnosti
- ovaj backlog je ulazni gate za `B20` i `B30`

## Alignment Note

Sledeca matrica je kanonski close-out `B10` zapisa za prvi runtime cut.

| Edge | Producer | Consumer | Delivery | Channel | Region | Payload |
| --- | --- | --- | --- | --- | --- | --- |
| `job_ingress -> state_core` | `job_ingress` | `state_core` | notify plus shared mailbox | `CNC_V2_CH_JOB_TO_STATE` | `job_submit` | `cnc_v2_job_submit_t` |
| `state_core -> planner` | `state_core` | `planner` | notify plus shared mailbox | `CNC_V2_CH_STATE_TO_PLANNER` | `planner_request` | `cnc_v2_planner_request_t` |
| `planner -> safety_coordinator` | `planner` | `safety_coordinator` | notify plus shared candidate | `CNC_V2_CH_PLANNER_TO_SAFETY` | `motion_candidate` | `cnc_v2_motion_candidate_t` |
| `safety_coordinator -> mcu_transport` | `safety_coordinator` | `mcu_transport` | notify plus shared decision | `CNC_V2_CH_SAFETY_TO_XPORT` | `safety_decision` | `cnc_v2_safety_decision_t` |
| `mcu_transport -> state_core` | `mcu_transport` | `state_core` | notify plus shared status | `CNC_V2_CH_XPORT_TO_STATE` | `transport_status` | `cnc_v2_transport_status_t` |
| `state_core -> session_store` | `state_core` | `session_store` | notify plus shared request | `CNC_V2_CH_STATE_TO_STORE` | `store_request` | `cnc_v2_store_request_t` |
| `state_core -> observability` | `state_core` | `observability` | notify plus shared snapshot | `CNC_V2_CH_STATE_TO_OBS` | `state_snapshot` | `cnc_v2_state_snapshot_t` |
| `planner -> observability` | `planner` | `observability` | notify plus shared status | `CNC_V2_CH_PLANNER_TO_OBS` | `planner_status` | `cnc_v2_planner_status_t` |
| `safety_coordinator -> observability` | `safety_coordinator` | `observability` | notify plus shared status | `CNC_V2_CH_SAFETY_TO_OBS` | `safety_status` | `cnc_v2_safety_status_t` |
| `mcu_transport -> observability` | `mcu_transport` | `observability` | notify plus shared status | `CNC_V2_CH_XPORT_TO_OBS` | `transport_status` | `cnc_v2_transport_status_t` |
| `session_store -> observability` | `session_store` | `observability` | notify plus shared status | `CNC_V2_CH_STORE_TO_OBS` | `store_status` | `cnc_v2_store_status_t` |

`observability_report` ostaje producer-owned output region za `observability`,
ne dodatni cross-PD command ili feedback edge.

## Work Items

### B10-001 Inventory canonical edge and region matrix

Zadatak:

- zatvoriti jednu matricu `edge -> channel -> region -> payload`

Acceptance:

- implementer vise ne mora da pogadja koji artifact nosi koji payload

Status:

- done

### B10-002 Freeze config blob layout contract

Zadatak:

- potvrditi da `meta.py` `struct.pack` layout-i odgovaraju
  `include/cnc_v2/config.h` packed struct-ovima

Acceptance:

- config blob ABI je 1:1 reviewable iz koda i generatora

Status:

- done

### B10-003 Freeze shared-memory publish/read contract

Zadatak:

- potvrditi da generation-based publish/read helper-i ostaju kanonski put za
  shared region-e

Acceptance:

- producer/consumer shared-memory semantics nisu prepuscene lokalnom tumacenju

Status:

- done

### B10-004 Freeze first board set and build graph

Zadatak:

- potvrditi da prvi runtime cut ima isti service graph na `qemu_virt_aarch64`
  i `rpi3b`

Acceptance:

- build report-i za obe putanje ne nose razlicit runtime graph

Status:

- done

### B10-005 Publish canonical close-out note

Zadatak:

- zadrzati ovaj backlog kao kanonski zapis sta je vec freeze-ovano pre daljeg
  bring-up i hardening rada

Acceptance:

- `B20` i `B30` mogu da krenu bez ponovnog otvaranja osnovnog channel/region
  modela

Status:

- done

## Out Of Scope

- retransmit ili bounded retry policy
- qemu ili board smoke acceptance
- persistence iza `session_store` stuba
- host UX, job format prosirenja ili planner feature creep

## Human-Owned Decisions

- promene kanonskog service graph-a ili authority modela posle ovog close-outa
- uvodjenje novih app PD-ova, novih region family-ja ili sireg Pi/MCU boundary-ja
