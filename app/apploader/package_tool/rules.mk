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

LIBCPPBOR_DIR := $(TRUSTY_TOP)/external/libcppbor

HOST_TOOL_NAME := apploader_package_tool

HOST_SRCS := \
	$(LOCAL_DIR)/apploader_package_tool.cpp \
	$(LOCAL_DIR)/../cose.cpp \
	$(LIBCPPBOR_DIR)/src/cppbor.cpp \
	$(LIBCPPBOR_DIR)/src/cppbor_parse.cpp \

HOST_INCLUDE_DIRS := \
	trusty/user/base/interface/apploader/include \
	external/libcppbor/include/cppbor \

# libcppbor checks if __TRUSTY__ is defined to determine whether it's linked
# into Android or Trusty; the library uses some Android-specific logging and
# other APIs that host tools don't provide, so we define __TRUSTY__ here to
# disable all the Android-specific code in libcppbor.
HOST_FLAGS := -D__TRUSTY__

# The COSE code also needs to use different APIs/macros for error printing
# depending on whether it's compiled for a host tool or Trusty application.
HOST_FLAGS += -D__COSE_HOST__

HOST_LIBS := \
	c++ \

# Build and statically link in boringssl so we don't have to worry about what
# version the host environment provides. OpenSSL 3.0 deprecates several of the
# low-level APIs used for trusty app signing and encryption.
HOST_DEPS := \
	trusty/user/base/host/boringssl

include make/host_tool.mk
