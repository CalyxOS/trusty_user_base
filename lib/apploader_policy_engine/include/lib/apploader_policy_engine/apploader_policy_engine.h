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

#pragma once

#include <lk/compiler.h>
#include <stdbool.h>
#include <uapi/trusty_uuid.h>

__BEGIN_CDECLS

/**
 * struct manifest_extracts - a subset of the manifest, that may
 *                            influence if app loading is allowed
 * @uuid: The UUID of the app.
 * @non_critical_app: Whether the app manifest opted-in to
 *                    NON_CRITICAL_APP.
 */
struct manifest_extracts {
    uuid_t uuid;
    bool non_critical_app;
};

/**
 * apploader_policy_engine_get_key() - Retrieves the public key indexed
 *                                     by the key ID, if policy permits.
 * @kid: Key ID.
 * @public_key_ptr: Public key in DER encoding will be stored here, if
 *                  retrieval is successful. If the call is successful,
 *                  the caller should call apploader_policy_engine_put_key()
 *                  on @public_key_ptr to dispose of the key.
 * @public_key_size_ptr: The size of the public key will be stored here, if
 *                       retrieval is successful.
 *
 * Returns: NO_ERROR if key retrieval is successful, assorted error codes
 * otherwise.
 */
int apploader_policy_engine_get_key(uint8_t kid,
                                    const uint8_t** public_key_ptr,
                                    unsigned int* public_key_size_ptr);

/**
 * apploader_policy_engine_put_key() - Dispose of a key that was returned
 *                                     by apploader_policy_engine_get_key().
 * @public_key_ptr: The public key that was returned by a successful call
 *                  to apploader_policy_engine_get_key().
 */
void apploader_policy_engine_put_key(const uint8_t* public_key_ptr);

/**
 * apploader_policy_engine_validate() - Check if app loading is allowed
 *                                      when using the specified combination
 *                                      of public key, UUID, and
 *                                      NON_CRITICAL_APP.
 * @public_key: Public key in DER encoding.
 * @public_key_size: The size of the public key.
 * @manifest_extracts: A subset of information from the manifest.
 *
 * Returns: true if app loading is allowed, false otherwise.
 */
int apploader_policy_engine_validate(
        const uint8_t* public_key,
        unsigned int public_key_size,
        struct manifest_extracts* manifest_extracts);

__END_CDECLS
