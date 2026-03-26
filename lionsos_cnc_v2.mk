#
# LionsOS CNC Control V2 build.
#
# SPDX-License-Identifier: MIT
#

TOP := ${LIONSOS_CNC_V2_DIR}
SDDF := $(LIONSOS)/dep/sddf
SUPPORTED_BOARDS := qemu_virt_aarch64 rpi3b
TOOLCHAIN ?= clang
export CCACHE_DISABLE ?= 1
export CNC_V2_UART_SCOPE_TEST ?= 0
SYSTEM_FILE := lionsos_cnc_v2.system
IMAGE_FILE := lionsos_cnc_v2.img
REPORT_FILE := report.txt
METAPROGRAM := $(TOP)/meta.py

APP_PD_NAMES := job_ingress state_core planner safety_coordinator mcu_transport session_store observability
IMAGES := \
	timer_driver.elf \
	serial_driver.elf \
	serial_virt_rx.elf \
	serial_virt_tx.elf \
	$(foreach pd,$(APP_PD_NAMES),$(pd).elf)

include ${SDDF}/tools/make/board/common.mk

CFLAGS += \
	-I$(TOP)/include \
	-I$(SDDF)/include \
	-I$(SDDF)/include/microkit \
	-DCNC_V2_UART_SCOPE_TEST=$(CNC_V2_UART_SCOPE_TEST)

SDDF_CUSTOM_LIBC := 1
LDFLAGS := -L$(BOARD_DIR)/lib
LIBS := --start-group -lmicrokit -Tmicrokit.ld libsddf_util_debug.a --end-group
include ${SDDF}/util/util.mk
include ${SDDF}/drivers/timer/${TIMER_DRIV_DIR}/timer_driver.mk
include ${SDDF}/drivers/serial/${UART_DRIV_DIR}/serial_driver.mk
include ${SDDF}/serial/components/serial_components.mk

$(IMAGES): libsddf_util_debug.a

HEADERS := \
	$(TOP)/include/cnc_v2/wire_protocol.h \
	$(TOP)/include/cnc_v2/control_ipc.h \
	$(TOP)/include/cnc_v2/runtime_topology.h \
	$(TOP)/include/cnc_v2/config.h \
	$(TOP)/include/cnc_v2/safety.h

define APP_PD_TEMPLATE
$(1).o: $(TOP)/components/$(1)/$(1).c $(HEADERS)
	$(CC) -c $(CFLAGS) $$< -o $$@

$(1).elf: $(1).o libsddf_util_debug.a
	$(LD) $(LDFLAGS) $$< $(LIBS) -o $$@
endef

$(foreach pd,$(APP_PD_NAMES),$(eval $(call APP_PD_TEMPLATE,$(pd))))

all: $(IMAGE_FILE)

$(SYSTEM_FILE): $(METAPROGRAM) $(IMAGES) $(DTB)
	PYTHONPATH=${SDDF}/tools/meta:$$PYTHONPATH $(PYTHON) $(METAPROGRAM) --sddf $(SDDF) --board $(MICROKIT_BOARD) --dtb $(DTB) --output . --sdf $(SYSTEM_FILE)
	$(OBJCOPY) --update-section .device_resources=serial_driver_device_resources.data serial_driver.elf
	$(OBJCOPY) --update-section .serial_driver_config=serial_driver_config.data serial_driver.elf
	$(OBJCOPY) --update-section .serial_virt_tx_config=serial_virt_tx.data serial_virt_tx.elf
	$(OBJCOPY) --update-section .serial_virt_rx_config=serial_virt_rx.data serial_virt_rx.elf
	$(OBJCOPY) --update-section .device_resources=timer_driver_device_resources.data timer_driver.elf
	$(OBJCOPY) --update-section .job_ingress_config=job_ingress.data job_ingress.elf
	$(OBJCOPY) --update-section .state_core_config=state_core.data state_core.elf
	$(OBJCOPY) --update-section .planner_config=planner.data planner.elf
	$(OBJCOPY) --update-section .safety_coordinator_config=safety_coordinator.data safety_coordinator.elf
	$(OBJCOPY) --update-section .mcu_transport_config=mcu_transport.data mcu_transport.elf
	$(OBJCOPY) --update-section .session_store_config=session_store.data session_store.elf
	$(OBJCOPY) --update-section .observability_config=observability.data observability.elf
	$(OBJCOPY) --update-section .serial_client_config=serial_client_mcu_transport.data mcu_transport.elf
	$(OBJCOPY) --update-section .timer_client_config=timer_client_state_core.data state_core.elf
	$(OBJCOPY) --update-section .timer_client_config=timer_client_mcu_transport.data mcu_transport.elf
	touch $@

$(IMAGE_FILE) $(REPORT_FILE): $(IMAGES) $(SYSTEM_FILE)
	$(MICROKIT_TOOL) $(SYSTEM_FILE) --search-path $(BUILD_DIR) --board $(MICROKIT_BOARD) --config $(MICROKIT_CONFIG) -o $(IMAGE_FILE) -r $(REPORT_FILE)

qemu: $(IMAGE_FILE)
	$(QEMU) -machine virt,virtualization=on \
		-cpu cortex-a53 \
		-serial mon:stdio \
		-device loader,file=$(IMAGE_FILE),addr=0x70000000,cpu-num=0 \
		-m size=2G \
		-nographic \
		-global virtio-mmio.force-legacy=false

qemu_stdio: qemu

clean::
	rm -f *.elf *.o *.data $(SYSTEM_FILE) $(IMAGE_FILE) $(REPORT_FILE) $(DTB)

clobber:: clean
	rm -f .board_cflags-* .serial_cflags-* .timer_cflags-*
