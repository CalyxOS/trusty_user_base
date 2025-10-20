# Copyright (C) 2025 The Android Open Source Project
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#

LOCAL_DIR := $(GET_LOCAL_DIR)

MODULE := $(LOCAL_DIR)

MODULE_AIDL_STABLE := true

MODULE_AIDL_VERSION := 1

GATEKEEPER_AIDL_DIR = hardware/interfaces/gatekeeper/aidl/aidl_api/android.hardware.gatekeeper/$(MODULE_AIDL_VERSION)
KEYMINT_AIDL_DIR := hardware/interfaces/security/keymint/aidl
SECURECLOCK_AIDL_DIR := hardware/interfaces/security/secureclock/aidl

MODULE_CRATE_NAME := android_hardware_gatekeeper

MODULE_AIDL_LANGUAGE := rust

MODULE_AIDL_PACKAGE := android/hardware/gatekeeper

MODULE_AIDL_INCLUDES := \
	-I $(GATEKEEPER_AIDL_DIR) \
	-I $(KEYMINT_AIDL_DIR) \
	-I $(SECURECLOCK_AIDL_DIR) \

MODULE_AIDL_FLAGS := \
	--stability=vintf \

MODULE_AIDLS := \
    $(GATEKEEPER_AIDL_DIR)/$(MODULE_AIDL_PACKAGE)/GatekeeperEnrollResponse.aidl    \
    $(GATEKEEPER_AIDL_DIR)/$(MODULE_AIDL_PACKAGE)/GatekeeperVerifyResponse.aidl    \
    $(GATEKEEPER_AIDL_DIR)/$(MODULE_AIDL_PACKAGE)/IGatekeeper.aidl                 \

MODULE_AIDL_RUST_DEPS := \
	android_hardware_security_keymint

MODULE_LIBRARY_DEPS := \
	hardware/interfaces/security/keymint/aidl \

include make/aidl.mk
