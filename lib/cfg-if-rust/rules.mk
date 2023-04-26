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

CFG_IF_SRC_DIR = $(call FIND_EXTERNAL,rust/crates/cfg-if)

MODULE_LIBRARY_DEPS += \
	trusty/user/base/lib/libcore-rust \
	trusty/user/base/lib/libcompiler_builtins-rust \

# NOTE: This crate is a dependency of libstd, so we can't implicitly depend on
# libstd as we would normally. The dependencies listed above are the ones
# declared in the `Cargo.toml` when the `rustc-dep-of-std` feature is enabled.
MODULE_ADD_IMPLICIT_DEPS := false

MODULE_SRCS := $(CFG_IF_SRC_DIR)/src/lib.rs

MODULE_CRATE_NAME := cfg_if

MODULE_RUST_EDITION := 2018

include make/library.mk
