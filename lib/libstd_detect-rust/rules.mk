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

SRC_DIR = $(RUST_BINDIR)/../lib/rustlib/src/rust/library/stdarch/crates/std_detect

MODULE_SRCS := $(SRC_DIR)/src/lib.rs

MODULE_CRATE_NAME := std_detect

MODULE_RUST_EDITION := 2018

MODULE_RUSTFLAGS += \
	--cfg 'feature="rustc-dep-of-std"' \

MODULE_LIBRARY_DEPS += \
	trusty/user/base/lib/libcore-rust \
	$(call FIND_CRATE,libc) \
	trusty/user/base/lib/liballoc-rust \
	trusty/user/base/lib/libcompiler_builtins-rust \
	$(call FIND_CRATE,cfg-if) \

MODULE_ADD_IMPLICIT_DEPS := false

# TODO: figure out why as of Rust 1.82, rustdoc fails nsjailed Soong builds
# whereas regular (build.py) builds still work.
MODULE_SKIP_DOCS := true

include make/library.mk
