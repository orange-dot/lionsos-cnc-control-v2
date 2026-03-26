<!--
     Copyright 2026

     SPDX-License-Identifier: MIT
-->

# B30 MCU Transport And Executor Hardening

## Svrha

Ovaj backlog hardenuje Pi/MCU wire boundary bez pomeranja authority modela ili
sirine runtime graph-a.

Granica backloga je namerno uska:

- cilj je da `mcu_transport` i `mcu/` dobiju jasan failure i recovery model
- cilj nije da MCU postane OS-like subsystem ili da planner/safety dobiju novi
  scope

## Status

Ovaj backlog je trenutno **planned**.

## Reality Check

Danas vec postoje:

- realan TX put kroz sDDF serial client queue
- RX byte-stream FSM nazad u `cnc_frame_t`
- timeout path koji degradira `transport_status`
- sinteticki MCU executor sa `ACK/NAK/status/heartbeat` odgovorima

Jos uvek nedostaje:

- eksplicitna response klasifikacija i korelacija sa pending command-om
- bounded retry ili abandon politika
- jasno razdvajanje heartbeat/liveness signala od command completion signala
- backlog-zakljucana semantika za to kako `state_core` i `observability`
  tumace NAK, timeout i unexpected valid response

## Chosen Defaults

Za prvi hardening cut vaze sledeci default-i:

- ostaje jedan in-flight command u transportu
- heartbeat ne zatvara pending command wait
- validan ali neocekivan response ne sme sam da obrise divergence ili timeout
- MCU executor ostaje sinteticki i bez internog task graph-a

## Definition Of Done

Ovaj backlog je gotov kada su istovremeno tacne sledece stvari:

- response klase (`ACK`, `NAK`, `POSITION`, `HEARTBEAT`, invalid/unexpected`)
  imaju jasnu semantiku
- timeout/retry/abandon pravila su eksplicitna i reviewable
- `transport_status` moze da razlikuje local queue fault, remote reject,
  timeout i protocol fault
- `state_core` i `observability` ne tretiraju sve validne RX frame-ove kao
  isti "success"
- MCU executor ostaje narrow bare-metal dispatcher

## Canonical Artifacts

- `docs/SUBSYSTEM_MCU_TRANSPORT.md`
- `docs/SUBSYSTEM_MCU_EXECUTOR.md`
- `docs/MCU_BOUNDARY.md`
- `include/cnc_v2/control_ipc.h`
- `include/cnc_v2/wire_protocol.h`
- `components/mcu_transport/mcu_transport.c`
- `components/state_core/state_core.c`
- `components/observability/observability.c`
- `mcu/include/cnc_mcu_v2/protocol.h`
- `mcu/src/main.c`
- `mcu/src/uart.c`

## Dependencies

- `B10` mora ostati zatvoren
- `B20` treba da zakljuca prvi canonical bring-up i smoke put pre agresivnijeg
  wire hardening rada

## Work Items

### B30-001 Freeze response classification model

Zadatak:

- zakljucati koji response tipovi:
  - zatvaraju pending command
  - samo nose liveness/status informaciju
  - predstavljaju protocol ili correlation problem

Acceptance:

- `mcu_transport` vise ne tretira svaki validan response kao isti `RESPONSE_OK`

Status:

- planned

### B30-002 Freeze timeout, retry and abandon policy

Zadatak:

- zakljucati da li prvi hardening cut ima:
  - zero retry
  - bounded retry
  - ili explicit abandon posle timeout-a
- policy mora ostati mali i reviewable

Acceptance:

- implementer ne mora sam da izmisli recovery ponasanje na wire fault-u

Status:

- planned

### B30-003 Harden invalid and unexpected RX handling

Zadatak:

- odvojiti:
  - CRC fault
  - unexpected response class
  - out-of-sequence response
- odrediti koji od tih slucajeva menjaju health, lifecycle ili samo evidence

Acceptance:

- protocol fault handling je eksplicitna, ne implicitna posledica trenutnog
  helper toka

Status:

- planned

### B30-004 Freeze heartbeat and status-query semantics

Zadatak:

- zakljucati kako heartbeat i position/status odgovori uticu na:
  - liveness
  - pending command wait
  - machine state publication

Acceptance:

- heartbeat i status response vise nisu "samo jos jedan validan frame"

Status:

- planned

### B30-005 Align transport consumers after hardening

Zadatak:

- uskladiti `state_core` lifecycle mapping i `observability` health/report
  model sa novim transport semantics

Acceptance:

- downstream potrosaci ne gube informaciju o tome da li je problem remote
  reject, timeout, CRC fault ili lokalni queue problem

Status:

- planned

## Out Of Scope

- promene planner ili safety policy scope-a
- prosirenje wire frame formata van onoga sto je potrebno za prvi hardening cut
- persistence, session semantics ili host UX
- uvodjenje MCU-side scheduler-a, task sistema ili dinamicke alokacije

## Human-Owned Decisions

- tacan retry budzet za prvi hardening cut
- da li `POSITION` response sme da zatvori pending command samo za
  `STATUS_QUERY` ili i za druge komande
- kako se remote `NAK` mapira na lifecycle/evidence model van uskog transport
  layer-a
