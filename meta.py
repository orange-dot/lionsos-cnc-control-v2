# Copyright 2026
# SPDX-License-Identifier: MIT
import argparse
import os
import struct
from dataclasses import dataclass
from typing import List

from importlib.metadata import version
from sdfgen import DeviceTree, Sddf, SystemDescription

assert version("sdfgen").split(".")[1] in ("28", "29"), "Unexpected sdfgen version"

ProtectionDomain = SystemDescription.ProtectionDomain
MemoryRegion = SystemDescription.MemoryRegion
Map = SystemDescription.Map
Channel = SystemDescription.Channel

CNC_V2_CONFIG_MAGIC = 0x32434E43

CH_JOB_TO_STATE = 0
CH_STATE_TO_PLANNER = 1
CH_PLANNER_TO_SAFETY = 2
CH_SAFETY_TO_XPORT = 3
CH_XPORT_TO_STATE = 4
CH_STATE_TO_STORE = 5
CH_STATE_TO_OBS = 6
CH_PLANNER_TO_OBS = 7
CH_SAFETY_TO_OBS = 8
CH_XPORT_TO_OBS = 9
CH_STORE_TO_OBS = 10

REGION_SIZE = 0x1000


@dataclass
class Board:
    name: str
    arch: SystemDescription.Arch
    paddr_top: int
    serial: str
    timer: str
    i2c: str | None = None


BOARDS: List[Board] = [
    Board(
        name="qemu_virt_aarch64",
        arch=SystemDescription.Arch.AARCH64,
        paddr_top=0x6_0000_000,
        serial="pl011@9000000",
        timer="timer",
    ),
    Board(
        name="rpi3b",
        arch=SystemDescription.Arch.AARCH64,
        paddr_top=0x0800_0000,
        serial="soc/serial@7e215040",
        timer="soc/timer@7e003000",
        i2c="soc/i2c@7e804000",
    ),
]


def add_map(pd: ProtectionDomain, mr: MemoryRegion, perms: str = "rw") -> int:
    mapping = Map(mr, pd.get_map_vaddr(mr), perms=perms)
    pd.add_map(mapping)
    return mapping.vaddr


def write_blob(output_dir: str, name: str, data: bytes) -> None:
    with open(f"{output_dir}/{name}", "wb") as blob:
        blob.write(data)


def serialise_app_configs(output_dir: str, configs: dict) -> None:
    write_blob(
        output_dir,
        "job_ingress.data",
        struct.pack("<IIQI", CNC_V2_CONFIG_MAGIC, 0, configs["job_ingress"]["job_submit"], CH_JOB_TO_STATE),
    )
    write_blob(
        output_dir,
        "state_core.data",
        struct.pack(
            "<IIQQQQQIII",
            CNC_V2_CONFIG_MAGIC,
            0,
            configs["state_core"]["job_submit"],
            configs["state_core"]["state_snapshot"],
            configs["state_core"]["planner_request"],
            configs["state_core"]["store_request"],
            configs["state_core"]["transport_status"],
            CH_STATE_TO_PLANNER,
            CH_STATE_TO_STORE,
            CH_STATE_TO_OBS,
        ),
    )
    write_blob(
        output_dir,
        "planner.data",
        struct.pack(
            "<IIQQQII",
            CNC_V2_CONFIG_MAGIC,
            0,
            configs["planner"]["planner_request"],
            configs["planner"]["motion_candidate"],
            configs["planner"]["planner_status"],
            CH_PLANNER_TO_SAFETY,
            CH_PLANNER_TO_OBS,
        ),
    )
    write_blob(
        output_dir,
        "safety_coordinator.data",
        struct.pack(
            "<IIQQQII",
            CNC_V2_CONFIG_MAGIC,
            0,
            configs["safety"]["motion_candidate"],
            configs["safety"]["safety_decision"],
            configs["safety"]["safety_status"],
            CH_SAFETY_TO_XPORT,
            CH_SAFETY_TO_OBS,
        ),
    )
    write_blob(
        output_dir,
        "mcu_transport.data",
        struct.pack(
            "<IIQQII",
            CNC_V2_CONFIG_MAGIC,
            0,
            configs["mcu_transport"]["safety_decision"],
            configs["mcu_transport"]["transport_status"],
            CH_XPORT_TO_STATE,
            CH_XPORT_TO_OBS,
        ),
    )
    write_blob(
        output_dir,
        "session_store.data",
        struct.pack(
            "<IIQQI",
            CNC_V2_CONFIG_MAGIC,
            0,
            configs["session_store"]["store_request"],
            configs["session_store"]["store_status"],
            CH_STORE_TO_OBS,
        ),
    )
    write_blob(
        output_dir,
        "observability.data",
        struct.pack(
            "<IIQQQQQQ",
            CNC_V2_CONFIG_MAGIC,
            0,
            configs["observability"]["state_snapshot"],
            configs["observability"]["planner_status"],
            configs["observability"]["safety_status"],
            configs["observability"]["transport_status"],
            configs["observability"]["store_status"],
            configs["observability"]["report"],
        ),
    )


def generate_cnc(sdf_path: str, output_dir: str, dtb: DeviceTree) -> None:
    serial_node = dtb.node(board.serial)
    timer_node = dtb.node(board.timer)
    assert serial_node is not None
    assert timer_node is not None

    timer_driver = ProtectionDomain("timer_driver", "timer_driver.elf", priority=254)
    timer_system = Sddf.Timer(sdf, timer_node, timer_driver)

    serial_driver = ProtectionDomain("serial_driver", "serial_driver.elf", priority=200)
    serial_virt_tx = ProtectionDomain("serial_virt_tx", "serial_virt_tx.elf", priority=199)
    serial_virt_rx = ProtectionDomain("serial_virt_rx", "serial_virt_rx.elf", priority=199)
    serial_system = Sddf.Serial(sdf, serial_node, serial_driver, serial_virt_tx, virt_rx=serial_virt_rx)

    job_ingress = ProtectionDomain("job_ingress", "job_ingress.elf", priority=140, budget=500, period=5000)
    state_core = ProtectionDomain("state_core", "state_core.elf", priority=252, budget=1000, period=2000)
    planner = ProtectionDomain("planner", "planner.elf", priority=220, budget=1000, period=5000)
    safety = ProtectionDomain("safety_coordinator", "safety_coordinator.elf", priority=253, budget=500, period=2000)
    mcu_transport = ProtectionDomain("mcu_transport", "mcu_transport.elf", priority=251, budget=1000, period=2000)
    session_store = ProtectionDomain("session_store", "session_store.elf", priority=120, budget=500, period=5000)
    observability = ProtectionDomain("observability", "observability.elf", priority=60, budget=500, period=20000)

    serial_system.add_client(mcu_transport)
    timer_system.add_client(state_core)
    timer_system.add_client(mcu_transport)

    regions = {}
    for name in [
        "job_submit",
        "state_snapshot",
        "planner_request",
        "motion_candidate",
        "planner_status",
        "safety_decision",
        "safety_status",
        "transport_status",
        "store_request",
        "store_status",
        "observability_report",
    ]:
        mr = MemoryRegion(sdf, name, REGION_SIZE)
        sdf.add_mr(mr)
        regions[name] = mr

    configs = {
        "job_ingress": {
            "job_submit": add_map(job_ingress, regions["job_submit"]),
        },
        "state_core": {
            "job_submit": add_map(state_core, regions["job_submit"], perms="r"),
            "state_snapshot": add_map(state_core, regions["state_snapshot"]),
            "planner_request": add_map(state_core, regions["planner_request"]),
            "store_request": add_map(state_core, regions["store_request"]),
            "transport_status": add_map(state_core, regions["transport_status"], perms="r"),
        },
        "planner": {
            "planner_request": add_map(planner, regions["planner_request"], perms="r"),
            "motion_candidate": add_map(planner, regions["motion_candidate"]),
            "planner_status": add_map(planner, regions["planner_status"]),
        },
        "safety": {
            "motion_candidate": add_map(safety, regions["motion_candidate"], perms="r"),
            "safety_decision": add_map(safety, regions["safety_decision"]),
            "safety_status": add_map(safety, regions["safety_status"]),
        },
        "mcu_transport": {
            "safety_decision": add_map(mcu_transport, regions["safety_decision"], perms="r"),
            "transport_status": add_map(mcu_transport, regions["transport_status"]),
        },
        "session_store": {
            "store_request": add_map(session_store, regions["store_request"], perms="r"),
            "store_status": add_map(session_store, regions["store_status"]),
        },
        "observability": {
            "state_snapshot": add_map(observability, regions["state_snapshot"], perms="r"),
            "planner_status": add_map(observability, regions["planner_status"], perms="r"),
            "safety_status": add_map(observability, regions["safety_status"], perms="r"),
            "transport_status": add_map(observability, regions["transport_status"], perms="r"),
            "store_status": add_map(observability, regions["store_status"], perms="r"),
            "report": add_map(observability, regions["observability_report"]),
        },
    }

    channels = [
        Channel(job_ingress, state_core, a_id=CH_JOB_TO_STATE, b_id=CH_JOB_TO_STATE),
        Channel(state_core, planner, a_id=CH_STATE_TO_PLANNER, b_id=CH_STATE_TO_PLANNER),
        Channel(planner, safety, a_id=CH_PLANNER_TO_SAFETY, b_id=CH_PLANNER_TO_SAFETY),
        Channel(safety, mcu_transport, a_id=CH_SAFETY_TO_XPORT, b_id=CH_SAFETY_TO_XPORT),
        Channel(mcu_transport, state_core, a_id=CH_XPORT_TO_STATE, b_id=CH_XPORT_TO_STATE),
        Channel(state_core, session_store, a_id=CH_STATE_TO_STORE, b_id=CH_STATE_TO_STORE),
        Channel(state_core, observability, a_id=CH_STATE_TO_OBS, b_id=CH_STATE_TO_OBS),
        Channel(planner, observability, a_id=CH_PLANNER_TO_OBS, b_id=CH_PLANNER_TO_OBS),
        Channel(safety, observability, a_id=CH_SAFETY_TO_OBS, b_id=CH_SAFETY_TO_OBS),
        Channel(mcu_transport, observability, a_id=CH_XPORT_TO_OBS, b_id=CH_XPORT_TO_OBS),
        Channel(session_store, observability, a_id=CH_STORE_TO_OBS, b_id=CH_STORE_TO_OBS),
    ]
    for ch in channels:
        sdf.add_channel(ch)

    for pd in [
        timer_driver,
        serial_driver,
        serial_virt_tx,
        serial_virt_rx,
        job_ingress,
        state_core,
        planner,
        safety,
        mcu_transport,
        session_store,
        observability,
    ]:
        sdf.add_pd(pd)

    assert serial_system.connect()
    assert serial_system.serialise_config(output_dir)
    assert timer_system.connect()
    assert timer_system.serialise_config(output_dir)
    serialise_app_configs(output_dir, configs)

    with open(f"{output_dir}/{sdf_path}", "w+") as f:
        f.write(sdf.render())


def generate_gimbal_demo(sdf_path: str, output_dir: str, dtb: DeviceTree) -> None:
    serial_node = dtb.node(board.serial)
    timer_node = dtb.node(board.timer)
    i2c_node = dtb.node(board.i2c) if board.i2c else None
    assert serial_node is not None
    assert timer_node is not None
    assert i2c_node is not None

    serial_driver = ProtectionDomain("serial_driver", "serial_driver.elf", priority=200)
    serial_virt_tx = ProtectionDomain("serial_virt_tx", "serial_virt_tx.elf", priority=199)
    timer_driver = ProtectionDomain("timer_driver", "timer_driver.elf", priority=180)
    i2c_driver = ProtectionDomain("i2c_driver", "i2c_driver.elf", priority=170)
    i2c_virt = ProtectionDomain("i2c_virt", "i2c_virt.elf", priority=169)
    micropython = ProtectionDomain("micropython", "micropython.elf", priority=80, budget=20000, stack_size=0x10000)

    gpio_regs = MemoryRegion(sdf, "gpio_regs", 0x1000, paddr=0x3F200000)
    sdf.add_mr(gpio_regs)
    i2c_driver.add_map(Map(gpio_regs, 0x30_100_000, "rw", cached=False))

    serial_system = Sddf.Serial(sdf, serial_node, serial_driver, serial_virt_tx, enable_color=False)
    timer_system = Sddf.Timer(sdf, timer_node, timer_driver)
    i2c_system = Sddf.I2c(sdf, i2c_node, i2c_driver, i2c_virt)

    serial_system.add_client(micropython)
    timer_system.add_client(micropython)
    i2c_system.add_client(micropython)

    for pd in [serial_driver, serial_virt_tx, timer_driver, i2c_driver, i2c_virt, micropython]:
        sdf.add_pd(pd)

    assert serial_system.connect()
    assert serial_system.serialise_config(output_dir)
    assert timer_system.connect()
    assert timer_system.serialise_config(output_dir)
    assert i2c_system.connect()
    assert i2c_system.serialise_config(output_dir)

    with open(f"{output_dir}/{sdf_path}", "w+") as f:
        f.write(sdf.render())


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--dtb", required=True)
    parser.add_argument("--sddf", required=True)
    parser.add_argument("--board", required=True, choices=[b.name for b in BOARDS])
    parser.add_argument("--output", required=True)
    parser.add_argument("--sdf", required=True)
    args = parser.parse_args()

    board = next(filter(lambda b: b.name == args.board, BOARDS))
    sdf = SystemDescription(board.arch, board.paddr_top)
    sddf = Sddf(args.sddf)

    with open(args.dtb, "rb") as f:
        dtb = DeviceTree(f.read())

    if args.board == "rpi3b" and bool(int(os.environ.get("CNC_V2_GIMBAL_DEMO", "0"))):
        generate_gimbal_demo(args.sdf, args.output, dtb)
    else:
        generate_cnc(args.sdf, args.output, dtb)
