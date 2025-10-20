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

BSSL_SRC_DIR := external/boringssl/src
BSSL_CRYPTO_DIR := $(BSSL_SRC_DIR)/rust/bssl-crypto

MODULE_SRCS := $(BSSL_CRYPTO_DIR)/src/lib.rs

MODULE_CRATE_NAME := bssl_crypto

MODULE_RUST_EDITION := 2021

MODULE_ADD_IMPLICIT_DEPS := false

MODULE_RUSTFLAGS += \
	--cfg 'feature="mlalgs"' \

MODULE_LIBRARY_DEPS += \
	trusty/user/base/lib/bssl-sys-rust \
	trusty/user/base/lib/libcore-rust \
	trusty/user/base/lib/liballoc-rust \
	trusty/user/base/lib/libcompiler_builtins-rust \

include make/library.mk
