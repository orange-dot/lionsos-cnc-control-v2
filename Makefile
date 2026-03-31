#
# LionsOS-native CNC/control starter wrapper.
#
# SPDX-License-Identifier: MIT
#

ifeq ($(strip $(MICROKIT_SDK)),)
$(error MICROKIT_SDK must be specified)
endif
override MICROKIT_SDK := $(abspath ${MICROKIT_SDK})

ifeq ($(strip $(LIONSOS)),)
$(error LIONSOS must be specified)
endif
export LIONSOS := $(abspath ${LIONSOS})
export LIONSOS_CNC_V2_DIR := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
export MICROKIT_CONFIG ?= debug
export BUILD_DIR ?= $(abspath build)
export MICROKIT_BOARD ?= qemu_virt_aarch64
export CNC_V2_UART_SCOPE_TEST ?= 0
export CNC_V2_GIMBAL_DEMO ?= 0

IMAGE_FILE := $(BUILD_DIR)/lionsos_cnc_v2.img
REPORT_FILE := $(BUILD_DIR)/report.txt

all: ${IMAGE_FILE}

qemu qemu_stdio ${IMAGE_FILE} ${REPORT_FILE} clean clobber: ${BUILD_DIR}/Makefile FORCE
	${MAKE} -C ${BUILD_DIR} MICROKIT_SDK=${MICROKIT_SDK} $(notdir $@)

${BUILD_DIR}/Makefile: lionsos_cnc_v2.mk Makefile
	mkdir -p ${BUILD_DIR}
	echo "export LIONSOS ?= ${LIONSOS}" > $@
	echo "export LIONSOS_CNC_V2_DIR ?= ${LIONSOS_CNC_V2_DIR}" >> $@
	echo "export BUILD_DIR := ${BUILD_DIR}" >> $@
	echo "export MICROKIT_BOARD ?= ${MICROKIT_BOARD}" >> $@
	echo "export MICROKIT_SDK ?= ${MICROKIT_SDK}" >> $@
	echo "export MICROKIT_CONFIG ?= ${MICROKIT_CONFIG}" >> $@
	echo "export CNC_V2_UART_SCOPE_TEST ?= ${CNC_V2_UART_SCOPE_TEST}" >> $@
	echo "export CNC_V2_GIMBAL_DEMO ?= ${CNC_V2_GIMBAL_DEMO}" >> $@
	cat lionsos_cnc_v2.mk >> $@

submodules:
	cd ${LIONSOS} && git submodule update --init dep/sddf
	cd ${LIONSOS} && git submodule update --init dep/musllibc

FORCE:
