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

MODULE_SRCS := $(call TOBUILDDIR,$(patsubst %.aidl,%.cpp,$(MODULE_AIDLS)))
AIDL_HEADER_DIR := $(BUILDDIR)/include
AIDL_TOOL := prebuilts/build-tools/linux-x86/bin/aidl
MODULE_AIDL_INCLUDES := $(foreach dir,$(sort $(foreach src,$(MODULE_AIDLS),$(dir $(src)))), -I $(dir))

# TODO: handle packages; for AIDL interfaces with package paths,
# the output directory for the tool should be at the root of
# the package path. The compiler creates one subdirectory
# per package component, e.g., com.foo.IFoo goes into com/foo/IFoo.cpp.
# Luckily the .aidl files are also required to follow this structure,
# so the input file is also com/foo/IFoo.aidl.
$(MODULE_SRCS): AIDL_TOOL := $(AIDL_TOOL)
$(MODULE_SRCS): AIDL_HEADER_DIR := $(AIDL_HEADER_DIR)
$(MODULE_SRCS): MODULE_AIDL_INCLUDES := $(MODULE_AIDL_INCLUDES)
$(MODULE_SRCS): MODULE_AIDL_FLAGS := $(MODULE_AIDL_FLAGS)
$(MODULE_SRCS): $(BUILDDIR)/%.cpp: %.aidl
	@$(MKDIR)
	@mkdir -p $(AIDL_HEADER_DIR)
	@echo generating $@ from AIDL
	$(NOECHO)$(AIDL_TOOL) --lang=cpp $(MODULE_AIDL_INCLUDES) -h $(AIDL_HEADER_DIR) -o $(dir $@) $(MODULE_AIDL_FLAGS) $<

MODULE_LIBRARY_DEPS += \
	trusty/user/base/lib/libstdc++-trusty \
	frameworks/native/libs/binder/trusty \

MODULE_EXPORT_INCLUDES += $(AIDL_HEADER_DIR)

# Build the AIDL module into a library
include make/library.mk

MODULE_AIDLS :=
MODULE_AIDL_INCLUDES :=
MODULE_AIDL_FLAGS :=
AIDL_HEADER_DIR :=
AIDL_TOOL :=