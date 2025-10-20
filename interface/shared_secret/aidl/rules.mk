# Copyright (C) 2023 The Android Open Source Project
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

SHARED_SECRET_AIDL_DIR = hardware/interfaces/security/sharedsecret/aidl/aidl_api/android.hardware.security.sharedsecret/$(MODULE_AIDL_VERSION)

MODULE_CRATE_NAME := android_hardware_security_sharedsecret

MODULE_AIDL_LANGUAGE := rust

MODULE_AIDL_PACKAGE := android/hardware/security/sharedsecret

MODULE_AIDL_INCLUDES := \
	-I $(SHARED_SECRET_AIDL_DIR) \

MODULE_AIDL_FLAGS := \
	--stability=vintf \

MODULE_AIDLS := \
    $(SHARED_SECRET_AIDL_DIR)/$(MODULE_AIDL_PACKAGE)/SharedSecretParameters.aidl     \
    $(SHARED_SECRET_AIDL_DIR)/$(MODULE_AIDL_PACKAGE)/ISharedSecret.aidl              \

include make/aidl.mk
