/*
 * Copyright (C) 2026 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#include <lib/hwkey/hwkey.h>
#include <stdbool.h>
#include <stdint.h>
#include <string.h>
#include <uapi/err.h>

static const hwkey_session_t mock_hwkey_session_handle = 42;

/*
 * Mock implementations of hwkey functions
 */

long hwkey_open(void) {
    /* Always return a fake session handle for success */
    return mock_hwkey_session_handle;
}

void hwkey_close(hwkey_session_t session) {
    (void)session;
    assert(session == mock_hwkey_session_handle);
}

long hwkey_get_keyslot_data(hwkey_session_t session,
                            const char* slot_id,
                            uint8_t* data,
                            uint32_t* data_size) {
    /* Mock implementation accepts any slot_id value */
    (void)slot_id;

    if (session != mock_hwkey_session_handle) {
        return ERR_BAD_HANDLE;
    }

    if (data == NULL || data_size == NULL) {
        return ERR_NOT_VALID;
    }

    /* Always succeed and return a buffer filled with zeros. */
    if (*data_size > 0) {
        memset(data, 0, *data_size); /* Clear the output buffer */
    }

    return NO_ERROR;
}

long hwkey_derive(hwkey_session_t session,
                  uint32_t* kdf_version,
                  const uint8_t* src,
                  uint8_t* dest,
                  uint32_t buf_size) {
    if (session != mock_hwkey_session_handle) {
        return ERR_BAD_HANDLE;
    }
    if (src == NULL || buf_size == 0 || dest == NULL || kdf_version == NULL) {
        /* Invalid input */
        return ERR_NOT_VALID;
    }
    if (*kdf_version > HWKEY_KDF_VERSION_1) {
        return ERR_NOT_VALID;
    }

    /*
     * The original function checks if `stored_buf_size != buf_size` after the
     * IPC call, returning `ERR_BAD_LEN`. This implies that the derived key
     * length must match the requested `buf_size`.
     */
    memset(dest, 0, buf_size);          /* Fill with zeros */
    *kdf_version = HWKEY_KDF_VERSION_1; /* Fixed KDF version */

    return NO_ERROR;
}

long hwkey_derive_versioned(hwkey_session_t session,
                            struct hwkey_versioned_key_options* args) {
    if (session != mock_hwkey_session_handle) {
        return ERR_BAD_HANDLE;
    }
    /* Replicate input validation from the original function */
    if (args == NULL) {
        return ERR_NOT_VALID;
    }
    if (args->context == NULL && args->context_len != 0) {
        return ERR_NOT_VALID;
    }
    if (args->context != NULL && args->context_len == 0) {
        return ERR_NOT_VALID;
    }
    if (args->key == NULL && args->key_len != 0) {
        return ERR_NOT_VALID;
    }
    if (args->key != NULL && args->key_len == 0) {
        return ERR_NOT_VALID;
    }
    if (args->kdf_version > HWKEY_KDF_VERSION_1) {
        return ERR_NOT_VALID;
    }
    if (args->os_rollback_version < 0 &&
        args->os_rollback_version != HWKEY_ROLLBACK_VERSION_CURRENT) {
        return ERR_NOT_VALID;
    }

    if (args->key_len > 0) {
        /* Only copy if a key buffer is provided */
        memset(args->key, 0, args->key_len); /* Fill with zeros */
    }

    /* Update output parameters in args */
    args->kdf_version = HWKEY_KDF_VERSION_1; /* Fixed KDF version */
    args->os_rollback_version = 0;           /* Fixed OS rollback version */

    return NO_ERROR;
}
