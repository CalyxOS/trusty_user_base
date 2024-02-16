# TODO: Move Trusty boringssl build into this repo, like we do for musl

OPENSSL_STUBS_DIR := $(GET_LOCAL_DIR)

MODULE_SRCS += \
	$(OPENSSL_STUBS_DIR)/rand.c \

MODULE_LIBRARY_DEPS += \
	trusty/user/base/lib/rng \
