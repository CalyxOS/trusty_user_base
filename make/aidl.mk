# Copyright (c) 2022, Google, Inc. All rights reserved
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

# Build an AIDL module for Trusty
#
# args:
# MODULE : module name (required)
# MODULE_AIDLS: list of AIDL files
# MODULE_AIDL_FLAGS: optional flags for the AIDL_TOOL binary
# MODULE_AIDL_PACKAGE: a path that matches the directory structure of the AIDL
#     package utilized in the module. For example, declaring
#     `package com.android.trusty.binder` should correspond to a
#     MODULE_AIDL_PACKAGE of com/android/trusty/binder. If MODULE_AIDL_PACKAGE
#     is not defined, it implies that there is no top-level `package`
#     declaration.

# Check that there are is at most one package specified
ifeq ($(filter $(words $(MODULE_AIDL_PACKAGE)),0 1),)
$(error $(MODULE) has the following packages $(MODULE_AIDL_PACKAGE), but only one is supported)
endif

# TODO: this implies all sources are under the same package; support multiple packages
MODULE_AIDL_INCLUDES ?=
MODULE_SRCS := $(call TOBUILDDIR,$(patsubst %.aidl,%.cpp,$(MODULE_AIDLS)))
AIDL_HEADER_DIR := $(BUILDDIR)/include
AIDL_TOOL := prebuilts/build-tools/linux-x86/bin/aidl
MODULE_AIDL_INCLUDES += $(subst $(if $(MODULE_AIDL_PACKAGE),/$(MODULE_AIDL_PACKAGE)/,),,$(foreach dir,$(sort $(foreach src,$(MODULE_AIDLS),$(dir $(src)))), -I $(dir)))

# TODO: support multiple, disparate packages; for AIDL interfaces with package paths,
# the output directory for the tool should be at the root of
# the package path. The compiler creates one subdirectory
# per package component, e.g., com.foo.IFoo goes into com/foo/IFoo.cpp.
# Luckily the .aidl files are also required to follow this structure,
# so the input file is also com/foo/IFoo.aidl.
$(MODULE_SRCS): MODULE_AIDL_PACKAGE := $(MODULE_AIDL_PACKAGE)
$(MODULE_SRCS): AIDL_TOOL := $(AIDL_TOOL)
$(MODULE_SRCS): AIDL_HEADER_DIR := $(AIDL_HEADER_DIR)
$(MODULE_SRCS): MODULE_AIDL_INCLUDES := $(MODULE_AIDL_INCLUDES)
$(MODULE_SRCS): MODULE_AIDL_FLAGS := $(MODULE_AIDL_FLAGS)
$(MODULE_SRCS): MODULE_AIDL_OUT_DIR := $(sort $(dir $(subst $(MODULE_AIDL_PACKAGE),,$(MODULE_SRCS))))
$(MODULE_SRCS): $(BUILDDIR)/%.cpp: %.aidl
	@$(MKDIR)
	@mkdir -p $(AIDL_HEADER_DIR)
	@echo generating $@ from AIDL
	$(NOECHO)$(AIDL_TOOL) --lang=cpp --structured $(MODULE_AIDL_INCLUDES) \
		-h $(AIDL_HEADER_DIR) -o $(MODULE_AIDL_OUT_DIR) $(MODULE_AIDL_FLAGS) $<

# AIDL generates .cpp files which depend on the binder and C++ modules
ifeq ($(call TOBOOL,$(TRUSTY_NEW_MODULE_SYSTEM)),false)
MODULE_DEPS += \
	trusty/kernel/lib/libcxx-trusty \
	frameworks/native/libs/binder/trusty/kernel
else
MODULE_LIBRARY_DEPS += \
	trusty/user/base/lib/libstdc++-trusty \
	frameworks/native/libs/binder/trusty
endif

MODULE_EXPORT_INCLUDES += $(AIDL_HEADER_DIR)

# Ensure that all auto-generated code, including headers, is
# emitted before downstream dependencies
MODULE_EXPORT_SRCDEPS += $(MODULE_SRCS)

# Build the AIDL module into a library
include make/library.mk

MODULE_AIDLS :=
MODULE_AIDL_INCLUDES :=
MODULE_AIDL_FLAGS :=
MODULE_AIDL_PACKAGE :=
AIDL_HEADER_DIR :=
AIDL_TOOL :=
