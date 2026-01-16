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
STORAGE_DIR := trusty/user/app/storage

MODULE := $(LOCAL_DIR)

MODULE_SRCS := system/see/authmgr/secure-storage-aidl-wrapper/rust/src/lib.rs

MODULE_CRATE_NAME := secure_storage_aidl_wrapper

MODULE_LIBRARY_EXPORTED_DEPS += \
	trusty/user/base/interface/secure_storage/rust \
	$(call FIND_CRATE,log) \
	$(call FIND_CRATE,thiserror) \

include $(STORAGE_DIR)/default_storage_flags.mk

# If storage supports staging changes during (possible) user data checkpointing,
# (like during an A/B Update) we won't be able to commit while there's an active
# checkpoint or before vold knows whether it needs to checkpoint. Stage changes
# instead so that writes will succeed, but not be committed until checkpointing
# completes.
ifeq (true,$(call TOBOOL,$(STORAGE_SUPPORT_STAGE_DURING_UDC)))
MODULE_RUSTFLAGS +=  --cfg 'feature="storage_support_stage_during_udc"'
endif

include make/library.mk
