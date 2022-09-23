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

LIBSTD_DIR = $(RUST_BINDIR)/../src/stdlibs/library/std

MODULE_SRCS := $(LIBSTD_DIR)/src/lib.rs

MODULE_CRATE_NAME := std

MODULE_RUST_EDITION := 2018

MODULE_LIBRARY_DEPS += \
	trusty/user/base/lib/libcore-rust \
	trusty/user/base/lib/libc-rust \
	trusty/user/base/lib/liballoc-rust \
	trusty/user/base/lib/libunwind-rust \
	trusty/user/base/lib/libhashbrown-rust \
	trusty/user/base/lib/libstd_detect-rust \
	trusty/user/base/lib/librustc-demangle-rust \
	trusty/user/base/lib/libpanic_abort-rust \

MODULE_RUSTFLAGS += \
	--cfg 'backtrace_in_libstd' \
	-Z force-unstable-if-unmarked \

# Suppress known warnings. The libstd source code generates warnings which are
# normally suppressed with `--cap-lints=allow` during the rustbuild process. We
# opt to suppress lints individually in order to avoid missing warnings that
# might be specific to the Trusty build process. These may need to be updated
# for future Rust releases.
MODULE_RUSTFLAGS += \
	-A unused-variables \
	-A non-fmt-panics \
	-A deprecated \

MODULE_ADD_IMPLICIT_DEPS := false

ifeq ($(ARCH),arm)
MODULE_RUST_ENV += STD_ENV_ARCH=arm
else ifeq ($(ARCH),arm64)
MODULE_RUST_ENV += STD_ENV_ARCH=aarch64
else
$(error Unrecognized arch $(ARCH), expected arm or arm64)
endif

MODULE_SKIP_DOCS := true

include make/library.mk
