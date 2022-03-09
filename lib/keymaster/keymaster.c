/*
 * Copyright (C) 2015 The Android Open Source Project
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

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <trusty_ipc.h>
#include <uapi/err.h>

#include <interface/keymaster/keymaster.h>
#include <lib/keymaster/keymaster.h>

#include <openssl/hmac.h>

#define LOG_TAG "libkeymaster"
#define TLOGE(fmt, ...) \
    fprintf(stderr, "%s: %d: " fmt, LOG_TAG, __LINE__, ##__VA_ARGS__)

#define HMAC_LEN (sizeof(((hw_auth_token_t*)0)->hmac))

static long send_req(keymaster_session_t session, uint32_t cmd) {
    struct keymaster_message msg = {
            .cmd = cmd,
    };

    struct iovec tx_iov = {
            .iov_base = &msg,
            .iov_len = sizeof(msg),
    };
    ipc_msg_t tx_msg = {
            .iov = &tx_iov,
            .num_iov = 1,
    };

    long rc = send_msg(session, &tx_msg);
    if (rc < 0) {
        TLOGE("%s: failed (%ld) to send_msg\n", __func__, rc);
        return rc;
    }

    if (((size_t)rc) != sizeof(msg)) {
        TLOGE("%s: msg invalid size (%zu != %zu)", __func__, (size_t)rc,
              sizeof(msg));
        return ERR_IO;
    }

    return NO_ERROR;
}

static long await_response(keymaster_session_t session,
                           struct ipc_msg_info* inf) {
    uevent_t uevt;
    long rc = wait(session, &uevt, INFINITE_TIME);
    if (rc != NO_ERROR) {
        TLOGE("%s: interrupted waiting for response (%ld)\n", __func__, rc);
        return rc;
    }

    rc = get_msg(session, inf);
    if (rc != NO_ERROR) {
        TLOGE("%s: failed to get_msg (%ld)\n", __func__, rc);
    }

    return rc;
}

/*
 * Reads a Keymaster Response message with a sized buffer. The format of the
 * response is as follows:
 *
 * command header : 4 bytes
 * error          : 4 bytes
 * data length    : 4 bytes
 * data           : |data length| bytes
 *
 * Returns 0 on success, a negative error code otherwise. On success, error is
 * filled. If error is zero, data and size are also filled.
 */
static int read_response(keymaster_session_t session,
                         struct ipc_msg_info* inf,
                         uint32_t cmd,
                         uint8_t** data,
                         uint32_t* size) {
    int rc;
    struct keymaster_message msg;
    struct km_raw_buffer_resp resp;
    size_t key_buf_len = 0;
    uint8_t* key_buf = NULL;

    if (inf->len > sizeof(msg) + sizeof(resp)) {
        key_buf_len = inf->len - sizeof(msg) - sizeof(resp);
        key_buf = malloc(key_buf_len);
        if (!key_buf) {
            TLOGE("%s: out of memory (%zu)\n", __func__, key_buf_len);
            return ERR_NO_MEMORY;
        }
    }

    struct iovec iov[] = {
            {.iov_base = &msg, .iov_len = sizeof(msg)},
            {.iov_base = &resp, .iov_len = sizeof(resp)},
            {.iov_base = key_buf, .iov_len = key_buf_len},
    };
    struct ipc_msg ipc_msg = {
            .iov = iov,
            .num_iov = countof(iov),
    };

    rc = read_msg(session, inf->id, 0, &ipc_msg);
    if (rc < 0) {
        TLOGE("%s: read_msg() failed: %d", __func__, rc);
        goto err;
    }

    if ((size_t)rc < sizeof(msg) + sizeof(resp.error)) {
        TLOGE("%s: unexpected number of bytes: %d\n", __func__, rc);
        rc = ERR_BAD_LEN;
        goto err;
    }

    /* Check cmd */
    if ((cmd | KM_RESP_BIT) != (msg.cmd & ~(KM_STOP_BIT))) {
        TLOGE("%s: invalid response (0x%x) for cmd (0x%x)\n", __func__, msg.cmd,
              cmd);
        rc = ERR_NOT_VALID;
        goto err;
    }

    /* We can return if there is an error */
    if (resp.error) {
        if ((size_t)rc != sizeof(msg) + sizeof(resp.error)) {
            TLOGE("%s: unexpected number of bytes: %d\n", __func__, rc);
            rc = ERR_BAD_LEN;
        }
        TLOGE("%s: KM response returned an error (%d)\n", __func__, resp.error);
        rc = ERR_GENERIC;
        goto err;
    }

    if ((size_t)rc < sizeof(msg) + sizeof(resp)) {
        TLOGE("%s: unexpected number of bytes: %d\n", __func__, rc);
        rc = ERR_BAD_LEN;
        goto err;
    }

    /* Check data_len */
    if (resp.data_size != key_buf_len) {
        TLOGE("%s: unexpected data length: %d\n", __func__, resp.data_size);
        rc = ERR_BAD_LEN;
        goto err;
    }

    if ((size_t)rc != sizeof(msg) + sizeof(resp) + resp.data_size) {
        TLOGE("%s: unexpected number of bytes: %d\n", __func__, rc);
        rc = ERR_BAD_LEN;
        goto err;
    }

    *data = key_buf;
    *size = resp.data_size;
    return NO_ERROR;

err:
    free(key_buf);
    return rc;
}

int keymaster_open(void) {
    return connect(KEYMASTER_SECURE_PORT, IPC_CONNECT_WAIT_FOR_PORT);
}

void keymaster_close(keymaster_session_t session) {
    close(session);
}

int keymaster_get_auth_token_key(keymaster_session_t session,
                                 uint8_t** key_buf_p,
                                 uint32_t* size_p) {
    if (size_p == NULL || key_buf_p == NULL) {
        return ERR_NOT_VALID;
    }

    long rc = send_req(session, KM_GET_AUTH_TOKEN_KEY);
    if (rc < 0) {
        TLOGE("%s: failed (%ld) to send req\n", __func__, rc);
        return rc;
    }

    struct ipc_msg_info inf;
    rc = await_response(session, &inf);
    if (rc < 0) {
        TLOGE("%s: failed (%ld) to await response\n", __func__, rc);
        return rc;
    }

    rc = read_response(session, &inf, KM_GET_AUTH_TOKEN_KEY, key_buf_p, size_p);
    put_msg(session, inf.id);
    if (rc < 0) {
        TLOGE("%s: failed read_response (%ld)\n", __func__, rc);
        return rc;
    }

    return NO_ERROR;
}

static int mint_hmac(uint8_t* key,
                     size_t key_size,
                     uint8_t* message,
                     size_t message_size,
                     uint8_t* hmac) {
    unsigned int tok_size;
    unsigned char* ret;
    memset(hmac, 0, HMAC_LEN);
    ret = HMAC(EVP_sha256(), (void*)key, key_size, message, message_size, hmac,
               &tok_size);
    if (ret == NULL || tok_size != HMAC_LEN) {
        TLOGE("Failed to execute HMAC()!\n");
        return ERR_FAULT;
    }

    return NO_ERROR;
}

int keymaster_sign_auth_token(keymaster_session_t session,
                              hw_auth_token_t* token) {
    int ret = NO_ERROR;

    if (token == NULL) {
        TLOGE("Invalid token!\n");
        return ERR_NOT_VALID;
    }

    uint8_t* key_buf;
    uint32_t key_buf_size;
    ret = keymaster_get_auth_token_key(session, &key_buf, &key_buf_size);
    if (ret) {
        return ret;
    }

    /* Initialize the token and message size */
    size_t message_size = sizeof(hw_auth_token_t) - sizeof(token->hmac);
    /* Mint the token key with the given HMAC key and message */
    ret = mint_hmac(key_buf, key_buf_size, (uint8_t*)token, message_size,
                    token->hmac);

free_mem:
    free(key_buf);
    return ret;
}

int keymaster_validate_auth_token(keymaster_session_t session,
                                  hw_auth_token_t* token) {
    int ret = NO_ERROR;

    if (token == NULL) {
        TLOGE("Invalid token!\n");
        return ERR_NOT_VALID;
    }

    uint8_t* key_buf;
    uint32_t key_buf_size;
    ret = keymaster_get_auth_token_key(session, &key_buf, &key_buf_size);
    if (ret) {
        return ret;
    }

    /* compute the expected token hmac */
    uint8_t expected_hmac[HMAC_LEN];
    size_t message_size = sizeof(hw_auth_token_t) - sizeof(token->hmac);

    ret = mint_hmac(key_buf, key_buf_size, (uint8_t*)token, message_size,
                    expected_hmac);
    if (ret) {
        goto free_mem;
    }

    /* Compare the expected hmac with the provided hmac */
    ret = memcmp(expected_hmac, token->hmac, sizeof(expected_hmac));

free_mem:
    free(key_buf);
    return ret;
}
