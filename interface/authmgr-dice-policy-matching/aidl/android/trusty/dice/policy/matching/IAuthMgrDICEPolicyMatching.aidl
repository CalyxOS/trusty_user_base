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

import android.trusty.dice.policy.matching.PolicyMatchingResult;
import android.trusty.dice.policy.matching.ClientMetaData;

/**
 * An internal AIDL interface implemented by AuthMgr-BE to provide the following services to the
 * trusted services:
 * 1) retrieving the latest DICE policy of a client (identified by the client sequence number)
 * 2) matching and comparing a given DICE policy of a client against the client's latest DICE chain
 *    and the policy respectively, that are held in the cache of the AuthMgr-BE.
 * 3) retrieving the metadata of a client (identified by the client sequence number)
 * If an error occurred in the invocation of any of the API methods, an error code defined in
 * Error.aidl or a generic binder error code will be returned as applicable.
 */
interface IAuthMgrDICEPolicyMatching {
    /**
     * Returns the latest DICE policy of the client identified by the client sequence number.
     * Note that the client should have been authorized with the AuthMgr-BE during the current boot
     * cycle in order for this method invocation to be successful.
     *
     * @param clientSeqNumber - a unique identifier assigned to a client by the AuthMgr BE
     *
     * @return: The latest DICE policy of the client. Note that we return a byte array here instead
     *          of the DicePolicy.aidl parcel type used in IAuthMgrAuthorization API because the
     *          parcel type requires a copy where we have the policy as a reference to a byte array.
     *          This byte array should encode the DICE policy according to the CDDL defined in:
     *          hardware/interfaces/security/authgraph/aidl/android/hardware/security/authgraph/
     *          DicePolicy.cddl
     */
    byte[] getPolicy(in long clientSeqNumber);

    /**
     * Returns the result of matching and comparing the given DICE policy of the client identified
     * by the clientSeqNumber against the latest DICE chain and the policy of the client
     * respectively.
     *
     * @param clientSeqNumber - a unique identifier assigned to a client by the AuthMgr BE
     *
     * @param dicePolicy - the DICE policy of the client to be matched against the latest DICE chain
     *                     of the client, and compared against the latest DICE policy of the client.
     *                     This byte array should encode the DICE policy according to the CDDL
     *                     defined in: hardware/interfaces/security/authgraph/aidl/android/hardware/
     *                     security/authgraph/DicePolicy.cddl
     *
     * @return result of matching the given DICE policy against the latest DICE chain of the client
     *         and comparing the given DICE policy with the latest DICE policy of the client
     */
    PolicyMatchingResult matchPolicy(in long clientSeqNumber, in byte[] dicePolicy);

    /**
     * Returns the metadata of the client identified by the client sequence number.
     *
     * @param clientSeqNumber - a unique identifier assigned to a client by the AuthMgr BE
     *
     * @return metadata of the client identified by the client sequence number.
     */
    ClientMetaData getClientMetaData(in long clientSeqNumber);
}
