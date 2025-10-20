/*
 * Copyright (C) 2025 The Android Open Source Project
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

package android.trusty.dice.policy.matching;

/**
 * Encapsulates the metadata of a client identified by the client sequence number, that is useful
 * for a trusted HAL service.
 */
parcelable ClientMetaData {
    /* Identifier of the pVM instance that the client is running in */
    byte[] instanceIdentifier;
    /* Identifier of the client assigned by the pVM instance */
    byte[] clientIdentifier;
    /* Whether or not the client is persistent */
    boolean isPersistent;
}