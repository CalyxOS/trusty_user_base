/*
 * Copyright 2025 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
package android.trusty.provisioning;

/**
 * Interface for provisioning device-specific attestation information.
 *
 * This interface is used by system components to provide unique device identifiers
 * to a secure environment. This information is typically used by TAs for device
 * attestation purposes.
 *
 * @hide
 */
interface IProvisioning {
    /**
     * Holds device-specific identifiers used for device attestation.
     *
     * All fields are byte vectors that typically hold UTF-8 string data.
     */
    parcelable AttestationIds {
        /** The device brand (e.g., "google"). */
        byte[] brand;
        /** The device name (e.g., "oriole"). */
        byte[] device;
        /** The product name (e.g., "oriole"). */
        byte[] product;
        /** The hardware serial number. */
        byte[] serial;
        /** The device IMEI. */
        byte[] imei;
        /** The second IMEI, if the device is dual-SIM. */
        byte[] imei2;
        /** The device MEID. */
        byte[] meid;
        /** The device manufacturer (e.g., "Google"). */
        byte[] manufacturer;
        /** The device model (e.g., "Pixel 6"). */
        byte[] model;
    }

    /**
     * Supported cryptographic algorithms for attestation keys.
     */
    enum SigningAlgorithm {
        /** RSA algorithm. */
        RSA,
        /** EC algorithm. */
        EC,
    }

    /**
     * Provides the device's unique attestation identifiers to a secure environment.
     * This method is expected to be called during the device's provisioning phase
     * to supply the necessary data for attestation features.
     *
     * @param attestIds A parcelable containing all the device identifiers.
     */
    void provisionAttestationIds(in AttestationIds attestIds);

    /**
     * Fetches the device's unique attestation identifiers from the secure storage
     *
     * @return AttestationIds on success.
     */
    AttestationIds getAttestationIds();

    /**
     * Provisions a cryptographic attestation key to the secure environment.
     *
     * This key serves as a fallback factory attestation key, used by the
     * Trusted Application (TA) to sign attestation certificates only when
     * Remote Key Provisioning (RKP) is not available on the device.
     *
     * @param algo The signing algorithm of the provided key.
     * @param key The private key material.
     */
    void provisionAttestationKey(in SigningAlgorithm algo, in byte[] key);

    /**
     * Appends a certificate chain to the collection of attestation chains stored
     * in the secure environment.
     *
     * This chain is associated with the fallback factory attestation key
     * (provisioned via {@link #provisionAttestationKey}) and is used by the
     * Trusted Application (TA) only when Remote Key Provisioning (RKP) is
     * not available.
     *
     * The secure environment supports storing multiple certificate chains. This
     * method adds the provided chain to that storage. If the storage file does
     * not exist, it will be created.
     *
     * @param algo The signing algorithm associated with this certificate chain.
     * This must match the algorithm of the provisioned attestation key.
     * @param certs The X.509 certificate chain to be stored.
     */
    void appendAttestationCertChain(in SigningAlgorithm algo, in byte[] certs);

}
