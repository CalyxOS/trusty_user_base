# This is included by library.mk to find the proper BUILDDIR location.
# Custom rules or other variables relative to BUILDDIR can also include it
# before including library.mk to get the correct BUILDDIR.
ifeq ($(call TOBOOL,$(TRUSTY_APP)),false)
ifeq ($(call TOBOOL,$(MODULE_RUST_HOST_LIB)),false)
BUILDDIR := $(TRUSTY_LIBRARY_BUILDDIR)
else
BUILDDIR := $(TRUSTY_HOST_LIBRARY_BUILDDIR)
endif
endif
