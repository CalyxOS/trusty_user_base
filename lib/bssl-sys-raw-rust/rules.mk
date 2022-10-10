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

MODULE_LIBRARY_DEPS += \
	trusty/user/base/lib/trusty-sys \
	external/boringssl \

MODULE_BINDGEN_SRC_HEADER := $(BSSL_SRC_DIR)/rust/wrapper.h

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

MODULE_INCLUDES += \
	$(BSSL_SRC_DIR)/include \

include make/library.mk
