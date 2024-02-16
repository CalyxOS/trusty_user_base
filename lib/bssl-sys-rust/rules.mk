# Copyright (C) 2022 The Android Open Source Project
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

BSSL_SRC_DIR := external/boringssl/src

# BoringSSL moved the sources under src/rust/bssl-sys,
# but they used to be one level higher so we use the old path
# for compatibility
BSSL_RUST_DIR := $(BSSL_SRC_DIR)/rust/bssl-sys
ifeq ($(wildcard $(BSSL_RUST_DIR)),)
BSSL_RUST_DIR := $(BSSL_SRC_DIR)/rust
endif

# Generate our `lib.rs` from the template provided with BoringSSL.
BSSL_LIB_RS_TEMPLATE := $(BSSL_RUST_DIR)/src/lib.rs
MODULE_LIB_RS_FILE := $(call TOBUILDDIR,$(BSSL_LIB_RS_TEMPLATE))
$(warning MODULE_LIB_RS_FILE $(MODULE_LIB_RS_FILE))
$(MODULE_LIB_RS_FILE): $(BSSL_LIB_RS_TEMPLATE)
	mkdir -p $(dir $@)
	cat "$(BSSL_LIB_RS_TEMPLATE)" > $@
	sed 's@^include!(env!(\"BINDGEN_RS_FILE\"));@pub use bssl_sys_raw::*;@' "$(BSSL_LIB_RS_TEMPLATE)" > $@

MODULE_SRCDEPS += $(MODULE_LIB_RS_FILE)

MODULE_SRCS := $(MODULE_LIB_RS_FILE)

MODULE_CRATE_NAME := bssl_ffi

MODULE_LIBRARY_DEPS += \
	trusty/user/base/lib/bssl-sys-raw-rust \
	trusty/user/base/lib/bssl-rust-support \

MODULE_INCLUDES += \
	$(BSSL_SRC_DIR)/include \

include make/library.mk
