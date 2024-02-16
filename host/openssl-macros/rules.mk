# Copyright (C) 2021 The Android Open Source Project
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

# temporarily handle both old and new crate paths (b/266828817)
ifneq ($(wildcard external/rust/crates/openssl-macros/.*),)
SRC_DIR := external/rust/crates/openssl-macros
else
SRC_DIR := external/rust/crates/rust-openssl/openssl-macros
endif

MODULE_SRCS := $(SRC_DIR)/src/lib.rs

MODULE_CRATE_NAME := openssl_macros

MODULE_RUST_EDITION := 2018

MODULE_LIBRARY_DEPS += \
	$(call FIND_CRATE,proc-macro2) \
	$(call FIND_CRATE,quote) \
	$(call FIND_CRATE,syn) \

MODULE_RUST_CRATE_TYPES := proc-macro

MODULE_SKIP_DOCS := true

include make/library.mk
