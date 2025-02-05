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

LIBCORE_DIR = $(RUST_BINDIR)/../lib/rustlib/src/rust/library/core

MODULE_SRCS := $(LIBCORE_DIR)/src/lib.rs

MODULE_CRATE_NAME := core

MODULE_RUST_EDITION := 2021

MODULE_RUSTFLAGS += \
	-Z force-unstable-if-unmarked \

# Allow targets to further reduce rust footprint by removing panic information
ifeq (true,$(call TOBOOL,$(RUST_USE_PANIC_IMMEDIATE_ABORT)))
MODULE_RUSTFLAGS += \
	--cfg 'feature="panic_immediate_abort"' \
	-A dead_code \

endif

MODULE_ADD_IMPLICIT_DEPS := false

# TODO: figure out why as of Rust 1.82, rustdoc fails nsjailed Soong builds
# whereas regular (build.py) builds still work fine.
MODULE_SKIP_DOCS := true

include make/library.mk
