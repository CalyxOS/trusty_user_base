/*
 * Copyright (C) 2024 The Android Open Source Project
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

/* AES benchmark using directly linked BoringSSL. */

#define TLOG_TAG "swaes_bench"
#include <assert.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>

#include <trusty_benchmark.h>
#include <trusty_unittest.h>
#include <uapi/err.h>

#include <openssl/evp.h>

#include "vectors.h"

/*
 * Define to verify crypto operation output matches the expected test vectors.
 * This adds overhead (memcmp()) to the benchmark, so is not normally desired.
 * However, it may be useful to verify that the correct cipher operation in
 * the benchmark.
 */
#define CHECK_RESULTS

/* Number of times to run the benchmark function with each parameter */
#define RUNS 40

/* test state */
struct crypto_swaes_state {
    const struct crypto_param* param;

    /* Encryption and decrypt context */
    EVP_CIPHER_CTX evp_ctx;

    /* Cipher to use, which combines mode and size */
    const EVP_CIPHER* cipher;

    /* Output buffer */
    void* buf;

    /* Tag buffer for GCM mode */
    uint8_t* tag;
};

static struct crypto_swaes_state* _state = NULL;

static struct crypto_param params[] = {
        /* Key and input sizes are given in bits
         *             mode, key, input, direction:
         */
        AES_CRYPT_ARGS(CBC, 128, 256, ENCRYPT),    /* 32 bytes */
        AES_CRYPT_ARGS(CBC, 128, 8192, ENCRYPT),   /*  1Kbytes */
        AES_CRYPT_ARGS(CBC, 128, 16384, ENCRYPT),  /*  2Kbytes */
        AES_CRYPT_ARGS(CBC, 128, 32768, ENCRYPT),  /*  4Kbytes */
        AES_CRYPT_ARGS(CBC, 128, 65536, ENCRYPT),  /*  8Kbytes */
        AES_CRYPT_ARGS(CBC, 128, 131072, ENCRYPT), /* 16Kbytes */

        AES_CRYPT_ARGS(CBC, 128, 256, DECRYPT),    /* 32 bytes */
        AES_CRYPT_ARGS(CBC, 128, 8192, DECRYPT),   /*  1Kbytes */
        AES_CRYPT_ARGS(CBC, 128, 16384, DECRYPT),  /*  2Kbytes */
        AES_CRYPT_ARGS(CBC, 128, 32768, DECRYPT),  /*  4Kbytes */
        AES_CRYPT_ARGS(CBC, 128, 65536, DECRYPT),  /*  8Kbytes */
        AES_CRYPT_ARGS(CBC, 128, 131072, DECRYPT), /* 16Kbytes */

        AES_CRYPT_ARGS(CBC, 256, 256, ENCRYPT),    /* 32 bytes */
        AES_CRYPT_ARGS(CBC, 256, 8192, ENCRYPT),   /*  1Kbytes */
        AES_CRYPT_ARGS(CBC, 256, 16384, ENCRYPT),  /*  2Kbytes */
        AES_CRYPT_ARGS(CBC, 256, 32768, ENCRYPT),  /*  4Kbytes */
        AES_CRYPT_ARGS(CBC, 256, 65536, ENCRYPT),  /*  8Kbytes */
        AES_CRYPT_ARGS(CBC, 256, 131072, ENCRYPT), /* 16Kbytes */

        AES_CRYPT_ARGS(CBC, 256, 256, DECRYPT),    /* 32 bytes */
        AES_CRYPT_ARGS(CBC, 256, 8192, DECRYPT),   /*  1Kbytes */
        AES_CRYPT_ARGS(CBC, 256, 16384, DECRYPT),  /*  2Kbytes */
        AES_CRYPT_ARGS(CBC, 256, 32768, DECRYPT),  /*  4Kbytes */
        AES_CRYPT_ARGS(CBC, 256, 65536, DECRYPT),  /*  8Kbytes */
        AES_CRYPT_ARGS(CBC, 256, 131072, DECRYPT), /* 16Kbytes */

        AES_CRYPT_ARGS(GCM, 128, 256, ENCRYPT),    /* 32 bytes */
        AES_CRYPT_ARGS(GCM, 128, 8192, ENCRYPT),   /*  1Kbytes */
        AES_CRYPT_ARGS(GCM, 128, 16384, ENCRYPT),  /*  2Kbytes */
        AES_CRYPT_ARGS(GCM, 128, 32768, ENCRYPT),  /*  4Kbytes */
        AES_CRYPT_ARGS(GCM, 128, 65536, ENCRYPT),  /*  8Kbytes */
        AES_CRYPT_ARGS(GCM, 128, 131072, ENCRYPT), /* 16Kbytes */

        AES_CRYPT_ARGS(GCM, 128, 256, DECRYPT),    /* 32 bytes */
        AES_CRYPT_ARGS(GCM, 128, 8192, DECRYPT),   /*  1Kbytes */
        AES_CRYPT_ARGS(GCM, 128, 16384, DECRYPT),  /*  2Kbytes */
        AES_CRYPT_ARGS(GCM, 128, 32768, DECRYPT),  /*  4Kbytes */
        AES_CRYPT_ARGS(GCM, 128, 65536, DECRYPT),  /*  8Kbytes */
        AES_CRYPT_ARGS(GCM, 128, 131072, DECRYPT), /* 16Kbytes */

        AES_CRYPT_ARGS(GCM, 256, 256, ENCRYPT),    /* 32 bytes */
        AES_CRYPT_ARGS(GCM, 256, 8192, ENCRYPT),   /*  1Kbytes */
        AES_CRYPT_ARGS(GCM, 256, 16384, ENCRYPT),  /*  2Kbytes */
        AES_CRYPT_ARGS(GCM, 256, 32768, ENCRYPT),  /*  4Kbytes */
        AES_CRYPT_ARGS(GCM, 256, 65536, ENCRYPT),  /*  8Kbytes */
        AES_CRYPT_ARGS(GCM, 256, 131072, ENCRYPT), /* 16Kbytes */

        AES_CRYPT_ARGS(GCM, 256, 256, DECRYPT),    /* 32 bytes */
        AES_CRYPT_ARGS(GCM, 256, 8192, DECRYPT),   /*  1Kbytes */
        AES_CRYPT_ARGS(GCM, 256, 16384, DECRYPT),  /*  2Kbytes */
        AES_CRYPT_ARGS(GCM, 256, 32768, DECRYPT),  /*  4Kbytes */
        AES_CRYPT_ARGS(GCM, 256, 65536, DECRYPT),  /*  8Kbytes */
        AES_CRYPT_ARGS(GCM, 256, 131072, DECRYPT), /* 16Kbytes */
};

static void get_param_name_cb(char* buf, size_t buf_size, size_t param_idx) {
    // TODO(b/330059594): param_idx goes out of bounds
    uint8_t cpu_idx = param_idx / countof(params);
    param_idx = param_idx % countof(params);
    snprintf(buf, buf_size, "cpu%u|%s%sK%zu_%zu", cpu_idx,
             params[param_idx].encrypt ? "ENC_" : "DEC_",
             params[param_idx].mode == AES_MODE_CBC ? "CBC_" : "GCM_",
             params[param_idx].key_size * 8, params[param_idx].input_size);
}

BENCH_SETUP(crypto) {
    const struct crypto_param* param = &params[bench_get_param_idx()];

    trusty_bench_get_param_name_cb = &get_param_name_cb;

    _state = calloc(sizeof(struct crypto_swaes_state), 1);
    ASSERT_NE(_state, NULL);

    _state->param = param;

    _state->buf = calloc(param->output_size, 1);
    ASSERT_NE(_state->buf, NULL);

    EVP_CIPHER_CTX_init(&_state->evp_ctx);

    switch (param->mode) {
    case AES_MODE_CBC:
        if (_state->param->key_size * 8 == 128) {
            _state->cipher = EVP_aes_128_cbc();
        } else if (_state->param->key_size * 8 == 256) {
            _state->cipher = EVP_aes_256_cbc();
        }
        break;
    case AES_MODE_GCM:
        if (_state->param->key_size * 8 == 128) {
            _state->cipher = EVP_aes_128_gcm();
        } else if (_state->param->key_size * 8 == 256) {
            _state->cipher = EVP_aes_256_gcm();
        }

        _state->tag = calloc(_state->param->tag_size, 1);
        ASSERT_NE(_state->tag, NULL);
        break;
    }

    ASSERT_NE(NULL, _state->cipher, "invalid cipher mode or size\n");

    /* Check the cipher parameters match the cipher */
    ASSERT_EQ(EVP_CIPHER_key_length(_state->cipher), param->key_size);
    ASSERT_EQ(EVP_CIPHER_iv_length(_state->cipher), param->iv_size);

    return NO_ERROR;
test_abort:
    return ERR_GENERIC;
}

BENCH_TEARDOWN(crypto) {
    EVP_CIPHER_CTX_cleanup(&_state->evp_ctx);
    if (_state->tag) {
        free(_state->tag);
    }
    free(_state->buf);
    free(_state);
}

static int encrypt(const struct crypto_param* param) {
    int rc, total_len, out_len = 0;

    rc = EVP_EncryptInit_ex(&_state->evp_ctx, _state->cipher, NULL, param->key,
                            param->iv);
    ASSERT_NE(0, rc, "EVP_EncryptInit_ex() failed\n");

    rc = EVP_CIPHER_CTX_set_padding(&_state->evp_ctx, 0);
    ASSERT_NE(0, rc, "EVP_CIPHER_CTX_set_padding() failed\n");

    if (param->mode == AES_MODE_GCM) {
        rc = EVP_EncryptUpdate(&_state->evp_ctx, NULL, &out_len, aad,
                               sizeof(aad));
        ASSERT_NE(0, rc, "EVP_EncryptUpdate(aad) failed\n");
        ASSERT_EQ(sizeof(aad), out_len);
    }

    rc = EVP_EncryptUpdate(&_state->evp_ctx, _state->buf, &out_len,
                           param->input, param->input_size);
    ASSERT_NE(0, rc, "EVP_EncryptUpdate failed\n");

    total_len = out_len;

    rc = EVP_EncryptFinal_ex(&_state->evp_ctx, _state->buf + total_len,
                             &out_len);
    ASSERT_NE(0, rc, "EVP_EncryptFinal_ex failed\n");

    total_len += out_len;
    ASSERT_EQ(total_len, param->output_size);

#ifdef CHECK_RESULTS
    ASSERT_EQ(0, memcmp(_state->buf, param->output, param->output_size),
              "ciphertext mismatch\n");

    if (param->mode == AES_MODE_GCM) {
        rc = EVP_CIPHER_CTX_ctrl(&_state->evp_ctx, EVP_CTRL_AEAD_GET_TAG,
                                 param->tag_size, _state->tag);
        ASSERT_NE(0, rc, "EVP_CIPHER_CTX_ctrl() failed\n");
        ASSERT_EQ(0, memcmp(_state->tag, param->tag, param->tag_size),
                  "tag mismatch\n");
    }
#endif
    EVP_CIPHER_CTX_reset(&_state->evp_ctx);

    return NO_ERROR;
test_abort:
    EVP_CIPHER_CTX_reset(&_state->evp_ctx);
    return ERR_GENERIC;
}

static int decrypt(const struct crypto_param* param) {
    int rc, total_len, out_len = 0;

    rc = EVP_DecryptInit_ex(&_state->evp_ctx, _state->cipher, NULL, param->key,
                            param->iv);

    ASSERT_NE(0, rc, "EVP_DecryptInit_ex() failed\n");

    rc = EVP_CIPHER_CTX_set_padding(&_state->evp_ctx, 0);
    ASSERT_NE(0, rc, "EVP_CIPHER_CTX_set_padding() failed\n");

    if (param->mode == AES_MODE_GCM) {
        rc = EVP_DecryptUpdate(&_state->evp_ctx, NULL, &out_len, aad,
                               sizeof(aad));
        ASSERT_NE(0, rc, "EVP_DecryptUpdate(aad) failed\n");
        ASSERT_EQ(sizeof(aad), out_len);

        rc = EVP_CIPHER_CTX_ctrl(&_state->evp_ctx, EVP_CTRL_AEAD_SET_TAG,
                                 param->tag_size, (void*)param->tag);
        ASSERT_NE(0, rc, "EVP_CIPHER_CTX_ctrl() failed\n");
    }

    rc = EVP_DecryptUpdate(&_state->evp_ctx, _state->buf, &out_len,
                           param->input, param->input_size);
    ASSERT_NE(0, rc, "EVP_DecryptUpdate failed\n");

    total_len = out_len;

    rc = EVP_DecryptFinal_ex(&_state->evp_ctx, _state->buf + out_len, &out_len);
    ASSERT_NE(0, rc, "EVP_DecryptFinal_ex failed\n");

    total_len += out_len;
    ASSERT_EQ(total_len, param->output_size);

#ifdef CHECK_RESULTS
    EXPECT_EQ(0, memcmp(_state->buf, param->output, param->output_size),
              "plaintext mismatch\n");
#endif

    EVP_CIPHER_CTX_reset(&_state->evp_ctx);

    return NO_ERROR;
test_abort:
    EVP_CIPHER_CTX_reset(&_state->evp_ctx);
    return ERR_GENERIC;
}

BENCH_ALL_CPU(crypto, swaes, RUNS, params) {
    const struct crypto_param* param = _state->param;

    return param->encrypt ? encrypt(param) : decrypt(param);
}

BENCH_RESULT(crypto, swaes, Mbit_s) {
    return (8000 * _state->param->input_size) / bench_get_duration_ns();
}

PORT_TEST(hwaes, "com.android.trusty.swaes.bench")
