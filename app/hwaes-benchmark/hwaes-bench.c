/*
 * Copyright (C) 2022 The Android Open Source Project
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

#define TLOG_TAG "hwaes_bench"
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>

#include <lib/hwaes/hwaes.h>
#include <lib/hwkey/hwkey.h>
#include <sys/auxv.h>
#include <sys/mman.h>
#include <trusty/memref.h>
#include <trusty_benchmark.h>
#include <trusty_unittest.h>
#include <uapi/err.h>

#include "vectors.h"

/*
 * Define to verify crypto operation output matches the expected test vectors.
 * This adds overhead (memcmp()) to the benchmark, so is not normally desired.
 * However, it may be useful to verify that the correct cipher operation in
 * the benchmark.
 */
// #define CHECK_RESULTS

/*
 * Define to make tests at buffer sizes greater than one page.
 * The driver & hardware must be able to support page crossings for these tests
 * to execute.
 */
// #define EXTENDED_BUFFERS

/* Number of times to run the benchmark function with each parameter */
#define RUNS 40

#define HWAES_MAX_NUM_HANDLES 8
#define AUX_PAGE_SIZE() getauxval(AT_PAGESZ)
#define CUR_PARAM params[bench_get_param_idx()]

/**
 * struct hwaes_iov - a wrapper of an array of iovec.
 * @iov: array of iovec.
 * @num_iov: number of iovec.
 * @total_len: total length of the tipc message.
 */
struct hwaes_iov {
    struct iovec iov[TIPC_MAX_MSG_PARTS];
    size_t num_iov;
    size_t total_len;
};

/**
 * struct hwaes_shm - a wrapper of an array of shared memory handles.
 * @handles:     array of shared memory handles.
 * @num_handles: number of shared memory handles.
 */
struct hwaes_shm {
    handle_t handles[HWAES_MAX_NUM_HANDLES];
    size_t num_handles;
};
/**
 * struct crypto_hwaes_state - holds the current bench state.
 * @hwaes_session: handle to an open session with the hwaes secure app.
 * @shm_hdin: shared memory handle for text in
 * @shm_hdout: shared memory handle for text out
 * @args:   parameters to the actual encryption routine
 * @req_hdr: request structure for hwaes. Holds the command to be sent
 * @cmd_hdr: request header for HWAES_AES command
 * @shm_descs: yet another packing of shared memory descriptor
 * @req_iov:   iovector array of requests
 */
struct crypto_hwaes_state {
    hwaes_session_t hwaes_session;
    struct hwcrypt_shm_hd shm_hdin;
    struct hwcrypt_shm_hd shm_hdout;
    struct hwcrypt_args args;
    struct hwaes_req req_hdr;
    struct hwaes_aes_req cmd_hdr;
    struct hwaes_shm_desc shm_descs[HWAES_MAX_NUM_HANDLES];
    struct hwaes_iov req_iov;
};

static struct crypto_hwaes_state* _state;

/**
 * struct crypto_hwaes_param - Necessary Parameters for hwaes_encrypt.
 * @key: key to use for encryption
 * @key_size: byte size of the key
 * @input: base address of the bytes blob to be encrypted/decrypted
 * @input_size: size of the bytes blob to be encrypted/decrypted
 * @output: bytes blob resulting from encryption/decryption
 * @output_size: size of the bytes blob resulting from encryption/decryption
 * @iv: initialization vector for encryption/decryption
 * @iv_size: size of the initialization vector for encryption/decryption
 * @tag: expected tag output for GCM encryption
 * @tag_size: size of the expected tag output for GCM encryption
 * @mode: GMC/CBC AES ecnryption block mode
 * @encrypt: direction? encrypt or decrypt
 */
struct crypto_hwaes_param {
    const uint8_t* key;
    size_t key_size;
    const uint8_t* input;
    size_t input_size;
    const uint8_t* output;
    size_t output_size;
    const uint8_t* iv;
    size_t iv_size;
    const uint8_t* tag;
    size_t tag_size;
    enum hwaes_mode mode;
    bool encrypt;
};

/**
 * params - Array of parameters for the parametric BENCH
 */
static struct crypto_hwaes_param params[] = {
        /* Key and input sizes are given in bits
         *                   mode, key, input, direction:
         */
        AES_CRYPT_ARGS(CBC, 128, 256, ENCRYPT),   /* 32 bytes */
        AES_CRYPT_ARGS(CBC, 128, 8192, ENCRYPT),  /*  1Kbytes */
        AES_CRYPT_ARGS(CBC, 128, 16384, ENCRYPT), /*  2Kbytes */
        AES_CRYPT_ARGS(CBC, 128, 32768, ENCRYPT), /*  4Kbytes */
#if EXTENDED_BUFFERS
        AES_CRYPT_ARGS(CBC, 128, 65536, ENCRYPT),  /*  8Kbytes */
        AES_CRYPT_ARGS(CBC, 128, 131072, ENCRYPT), /* 16Kbytes */
#endif
        AES_CRYPT_ARGS(CBC, 128, 256, DECRYPT),   /* 32 bytes */
        AES_CRYPT_ARGS(CBC, 128, 8192, DECRYPT),  /*  1Kbytes */
        AES_CRYPT_ARGS(CBC, 128, 16384, DECRYPT), /*  2Kbytes */
        AES_CRYPT_ARGS(CBC, 128, 32768, DECRYPT), /*  4Kbytes */
#if EXTENDED_BUFFERS
        AES_CRYPT_ARGS(CBC, 128, 65536, DECRYPT),  /*  8Kbytes */
        AES_CRYPT_ARGS(CBC, 128, 131072, DECRYPT), /* 16Kbytes */
#endif
        AES_CRYPT_ARGS(CBC, 256, 256, ENCRYPT),   /* 32 bytes */
        AES_CRYPT_ARGS(CBC, 256, 8192, ENCRYPT),  /*  1Kbytes */
        AES_CRYPT_ARGS(CBC, 256, 16384, ENCRYPT), /*  2Kbytes */
        AES_CRYPT_ARGS(CBC, 256, 32768, ENCRYPT), /*  4Kbytes */
#if EXTENDED_BUFFERS
        AES_CRYPT_ARGS(CBC, 256, 65536, ENCRYPT),  /*  8Kbytes */
        AES_CRYPT_ARGS(CBC, 256, 131072, ENCRYPT), /* 16Kbytes */
#endif
        AES_CRYPT_ARGS(CBC, 256, 256, DECRYPT),   /* 32 bytes */
        AES_CRYPT_ARGS(CBC, 256, 8192, DECRYPT),  /*  1Kbytes */
        AES_CRYPT_ARGS(CBC, 256, 16384, DECRYPT), /*  2Kbytes */
        AES_CRYPT_ARGS(CBC, 256, 32768, DECRYPT), /*  4Kbytes */
#if EXTENDED_BUFFERS
        AES_CRYPT_ARGS(CBC, 256, 65536, DECRYPT),  /*  8Kbytes */
        AES_CRYPT_ARGS(CBC, 256, 131072, DECRYPT), /* 16Kbytes */
#endif
        AES_CRYPT_ARGS(GCM, 128, 256, ENCRYPT),   /* 32 bytes */
        AES_CRYPT_ARGS(GCM, 128, 8192, ENCRYPT),  /*  1Kbytes */
        AES_CRYPT_ARGS(GCM, 128, 16384, ENCRYPT), /*  2Kbytes */
        AES_CRYPT_ARGS(GCM, 128, 32768, ENCRYPT), /*  4Kbytes */
#if EXTENDED_BUFFERS
        AES_CRYPT_ARGS(GCM, 128, 65536, ENCRYPT),  /*  8Kbytes */
        AES_CRYPT_ARGS(GCM, 128, 131072, ENCRYPT), /* 16Kbytes */
#endif
        AES_CRYPT_ARGS(GCM, 128, 256, DECRYPT),   /* 32 bytes */
        AES_CRYPT_ARGS(GCM, 128, 8192, DECRYPT),  /*  1Kbytes */
        AES_CRYPT_ARGS(GCM, 128, 16384, DECRYPT), /*  2Kbytes */
        AES_CRYPT_ARGS(GCM, 128, 32768, DECRYPT), /*  4Kbytes */
#if EXTENDED_BUFFERS
        AES_CRYPT_ARGS(GCM, 128, 65536, DECRYPT),  /*  8Kbytes */
        AES_CRYPT_ARGS(GCM, 128, 131072, DECRYPT), /* 16Kbytes */
#endif
        AES_CRYPT_ARGS(GCM, 256, 256, ENCRYPT),   /* 32 bytes */
        AES_CRYPT_ARGS(GCM, 256, 8192, ENCRYPT),  /*  1Kbytes */
        AES_CRYPT_ARGS(GCM, 256, 16384, ENCRYPT), /*  2Kbytes */
        AES_CRYPT_ARGS(GCM, 256, 32768, ENCRYPT), /*  4Kbytes */
#if EXTENDED_BUFFERS
        AES_CRYPT_ARGS(GCM, 256, 65536, ENCRYPT),  /*  8Kbytes */
        AES_CRYPT_ARGS(GCM, 256, 131072, ENCRYPT), /* 16Kbytes */
#endif
        AES_CRYPT_ARGS(GCM, 256, 256, DECRYPT),   /* 32 bytes */
        AES_CRYPT_ARGS(GCM, 256, 8192, DECRYPT),  /*  1Kbytes */
        AES_CRYPT_ARGS(GCM, 256, 16384, DECRYPT), /*  2Kbytes */
        AES_CRYPT_ARGS(GCM, 256, 32768, DECRYPT), /*  4Kbytes */
#if EXTENDED_BUFFERS
        AES_CRYPT_ARGS(GCM, 256, 65536, DECRYPT),  /*  8Kbytes */
        AES_CRYPT_ARGS(GCM, 256, 131072, DECRYPT), /* 16Kbytes */
#endif
};

static void get_param_name_cb(char* buf, size_t buf_size, size_t param_idx) {
    snprintf(buf, buf_size, "%s%sK%zu_%zu",
             params[param_idx].encrypt ? "ENC_" : "DEC_",
             params[param_idx].mode == HWAES_CBC_MODE ? "CBC_" : "GCM_",
             params[param_idx].key_size * 8, params[param_idx].input_size * 8);
}

static int shm_alloc(size_t size, struct hwcrypt_shm_hd* shm_hd) {
    memset(shm_hd, 0, sizeof(struct hwcrypt_shm_hd));

    void* base = memalign(AUX_PAGE_SIZE(), size);
    if (base == NULL) {
        return ERR_NO_MEMORY;
    }

    handle_t handle =
            (handle_t)memref_create(base, size, PROT_READ | PROT_WRITE);
    if (handle < 0) {
        return ERR_BAD_HANDLE;
    }

    shm_hd->handle = handle;
    shm_hd->base = base;
    shm_hd->size = size;

    return NO_ERROR;
}

static void shm_free(struct hwcrypt_shm_hd* shm_hd) {
    if (shm_hd->base) {
        close(shm_hd->handle);
        free((void*)shm_hd->base);

        shm_hd->base = NULL;
    }
}

BENCH_SETUP(crypto) {
    int rc;

    trusty_bench_get_param_name_cb = &get_param_name_cb;

    _state = calloc(sizeof(struct crypto_hwaes_state), 1);
    ASSERT_NE(NULL, _state, "calloc() failed\n");

    _state->hwaes_session = INVALID_IPC_HANDLE;

    size_t size = round_up(CUR_PARAM.input_size + GCM_TAG_LEN, AUX_PAGE_SIZE());

    rc = shm_alloc(size, &_state->shm_hdin);
    ASSERT_EQ(rc, NO_ERROR);
    rc = shm_alloc(size, &_state->shm_hdout);
    ASSERT_EQ(rc, NO_ERROR);

    /*
     * Clear the shared memory and fill it with appropriate plaintext/ciphertext
     */
    ASSERT_GE(_state->shm_hdin.size, CUR_PARAM.input_size);
    memset((void*)_state->shm_hdin.base, 0, _state->shm_hdin.size);
    memcpy((void*)_state->shm_hdin.base, CUR_PARAM.input, CUR_PARAM.input_size);

    /*
     * Setup the allocated state and open session with hwaes trusted app server
     */
    rc = hwaes_open(&_state->hwaes_session);
    ASSERT_EQ(rc, NO_ERROR);

    /*
     * Pack the required arguments for hwaes_encrypt/hwaes_decrypt.
     */
    _state->args = (struct hwcrypt_args){
            .key =
                    {
                            .data_ptr = CUR_PARAM.key,
                            .len = CUR_PARAM.key_size,
                    },
            .iv =
                    {
                            .data_ptr = CUR_PARAM.iv,
                            .len = CUR_PARAM.iv_size,
                    },
            .text_in =
                    {
                            .data_ptr = (void*)_state->shm_hdin.base,
                            .len = CUR_PARAM.input_size,
                            .shm_hd_ptr = &_state->shm_hdin,
                    },
            .text_out =
                    {
                            .data_ptr = (void*)_state->shm_hdout.base,
                            .len = CUR_PARAM.output_size,
                            .shm_hd_ptr = &_state->shm_hdout,
                    },
            .key_type = HWAES_PLAINTEXT_KEY,
            .padding = HWAES_NO_PADDING,
            .mode = CUR_PARAM.mode,
    };

    if (CUR_PARAM.mode == HWAES_GCM_MODE) {
        _state->args.aad.data_ptr = aad;
        _state->args.aad.len = sizeof(aad);

        if (CUR_PARAM.encrypt) {
            EXPECT_GE(_state->shm_hdout.size,
                      _state->args.text_out.len + GCM_TAG_LEN);

            _state->args.tag_out.len = GCM_TAG_LEN;
            _state->args.tag_out.data_ptr =
                    (void*)_state->shm_hdout.base + _state->args.text_out.len;
            _state->args.tag_out.shm_hd_ptr = &_state->shm_hdout;
        } else {
            _state->args.tag_in.len = GCM_TAG_LEN;
            _state->args.tag_in.data_ptr = CUR_PARAM.tag;
        }
    }

    /*
     * Prepare the command for hwaes server app.
     */
    _state->req_hdr = (struct hwaes_req){
            .cmd = HWAES_AES,
    };
    _state->cmd_hdr = (struct hwaes_aes_req){
            .key =
                    (struct hwaes_data_desc){
                            .len = CUR_PARAM.key_size,
                            .shm_idx = 0,
                    },
            .num_handles = 2,
    };
    _state->shm_descs[0] =
            (struct hwaes_shm_desc){.size = _state->shm_hdin.size};
    _state->shm_descs[1] =
            (struct hwaes_shm_desc){.size = _state->shm_hdout.size};
    _state->req_iov = (struct hwaes_iov){
            .iov =
                    {
                            {&_state->req_hdr, sizeof(_state->req_hdr)},
                            {&_state->cmd_hdr, sizeof(_state->cmd_hdr)},
                            {&_state->shm_descs,
                             sizeof(struct hwaes_shm_desc) * 2},
                    },
            .num_iov = 3,
            .total_len = sizeof(_state->req_hdr) + sizeof(_state->cmd_hdr) +
                         sizeof(struct hwaes_shm_desc) * 2,
    };

    return NO_ERROR;

test_abort:
    if (_state) {
        shm_free(&_state->shm_hdin);
        shm_free(&_state->shm_hdout);

        if (_state->hwaes_session != INVALID_IPC_HANDLE) {
            close(_state->hwaes_session);
        }
        free(_state);
    }

    return ERR_GENERIC;
}

BENCH_TEARDOWN(crypto) {
    close(_state->hwaes_session);
    shm_free(&_state->shm_hdin);
    shm_free(&_state->shm_hdout);
    free(_state);
}

static int encrypt(void) {
    int rc = hwaes_encrypt(_state->hwaes_session, &_state->args);

    ASSERT_EQ(HWAES_NO_ERROR, rc, "encryption failed for param: %zu\n",
              bench_get_param_idx());

#ifdef CHECK_RESULTS
    ASSERT_EQ(0,
              memcmp(_state->args.text_out.data_ptr, CUR_PARAM.output,
                     CUR_PARAM.output_size),
              "cipher-text mismatch for param: %zu\n", bench_get_param_idx());

    /* Verify the tag if used (GCM mode) */
    if (CUR_PARAM.mode == HWAES_GCM_MODE) {
        ASSERT_EQ(0,
                  memcmp(_state->args.tag_out.data_ptr, CUR_PARAM.tag,
                         CUR_PARAM.tag_size),
                  "tag mismatch for param: %zu\n", bench_get_param_idx());
    }
#endif
test_abort:
    return rc;
}

static int decrypt(void) {
    int rc = hwaes_decrypt(_state->hwaes_session, &_state->args);

    ASSERT_EQ(HWAES_NO_ERROR, rc, "decryption failed for param: %zu\n",
              bench_get_param_idx());
#ifdef CHECK_RESULTS
    ASSERT_EQ(0,
              memcmp(_state->args.text_out.data_ptr, CUR_PARAM.output,
                     CUR_PARAM.output_size),
              "cipher-text mismatch for param: %zu\n", bench_get_param_idx());
#endif
test_abort:
    return rc;
}

BENCH(crypto, hwaes, RUNS, params) {
    return CUR_PARAM.encrypt ? encrypt() : decrypt();
}

BENCH_RESULT(crypto, hwaes, Kbit_s) {
    return (8000000 * CUR_PARAM.input_size) / bench_get_duration_ns();
}

BENCH_RESULT(crypto, hwaes, Mbit_s) {
    return (8000 * CUR_PARAM.input_size) / bench_get_duration_ns();
}

PORT_TEST(hwaes, "com.android.trusty.hwaes.bench")
