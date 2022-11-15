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

PROTOBUF_DIR := external/rust/crates/protobuf

MODULE_SRCS := $(PROTOBUF_DIR)/src/lib.rs

MODULE_CRATE_NAME := protobuf

MODULE_RUST_EDITION := 2018

VERSION_OUT_FILE := $(call TOBUILDDIR,$(MODULE))/version.rs
$(VERSION_OUT_FILE): $(PROTOBUF_DIR)/out/version.rs
	@echo copying $< to $@
	@$(MKDIR)
	cp $< $@

MODULE_SRCDEPS += $(VERSION_OUT_FILE)

MODULE_RUST_ENV += OUT_DIR=$(dir $(VERSION_OUT_FILE))

include make/library.mk
