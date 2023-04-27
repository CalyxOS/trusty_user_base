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

MODULE_SRCS := $(LOCAL_DIR)/src/lib.rs

MODULE_CRATE_NAME := bssl_sys_raw

BSSL_SRC_DIR := external/boringssl/src

# BoringSSL moved the sources under src/rust/bssl-sys,
# but they used to be one level higher so we use the old path
# for compatibility
BSSL_RUST_DIR := $(BSSL_SRC_DIR)/rust/bssl-sys
ifeq ($(wildcard $(BSSL_RUST_DIR)),)
BSSL_RUST_DIR := $(BSSL_SRC_DIR)/rust
endif

MODULE_LIBRARY_DEPS += \
	trusty/user/base/lib/trusty-sys \
	external/boringssl \

MODULE_BINDGEN_SRC_HEADER := $(BSSL_RUST_DIR)/wrapper.h

MODULE_BINDGEN_FLAGS += \
	--no-derive-default \
	--enable-function-attribute-detection \
	--default-macro-constant-type="signed" \
	--rustified-enum="point_conversion_form_t" \

# These are not BoringSSL symbols, they are from glibc
# and are not relevant to the build besides throwing warnings
# about their 'long double' (aka u128) not being FFI safe.
# We block those functions so that the build doesn't
# spam warnings.
#
# https://github.com/rust-lang/rust-bindgen/issues/1549 describes the current problem
# and other folks' solutions.
MODULE_BINDGEN_FLAGS += \
	--blocklist-function="strtold" \
	--blocklist-function="qecvt" \
	--blocklist-function="qecvt_r" \
	--blocklist-function="qgcvt" \
	--blocklist-function="qfcvt" \
	--blocklist-function="qfcvt_r" \

# Specifying the correct clang target results in __builtin_va_list being
# declared as a 4 item array of u64 for aarch64 targets. This is not FFI-safe,
# so we can't declare va_list functions for aarch64 until bindgen supports
# mapping va_list to its Rust equivalent
# (https://github.com/rust-lang/rust/issues/44930)
MODULE_BINDGEN_FLAGS += \
	--blocklist-function="v.*printf.*" \
	--blocklist-function="v.*scanf.*" \
	--blocklist-function="BIO_vsnprintf" \
	--blocklist-function="OPENSSL_vasprintf" \

MODULE_INCLUDES += \
	$(BSSL_SRC_DIR)/include \

include make/library.mk
