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

//! This module contains helpers for trusty TAs that need to provide or test against
//! VINTF stable services in trusty. It's pretty unlikely that you should be using
//! these helpers unless you're implementing a TA that facilitates access to services
//! like authmgr-fe or the VINTF testing TA.

use binder::{SpIBinder, StatusCode};
use crate::wait_for_interface;
use android_hardware_security_see_authmgr::aidl::android::hardware::security::see::authmgr::IAuthMgrAuthorization::IAuthMgrAuthorization;
use android_hardware_security_see_devicestate::aidl::android::hardware::security::see::devicestate::IDeviceState::IDeviceState;
use android_hardware_security_see_hwcrypto::aidl::android::hardware::security::see::hwcrypto::IHwCryptoKey::IHwCryptoKey;
use android_hardware_security_see_storage::aidl::android::hardware::security::see::storage::ISecureStorage::ISecureStorage;

const SECURE_STORAGE_SERVICE_NAME: &str =
    "android.hardware.security.see.storage.ISecureStorage/default";
const HWCRYPTO_KEY_SERVICE_NAME: &str =
    "android.hardware.security.see.hwcrypto.IHwCryptoKey/default";
const AUTHMGR_SERVICE_NAME: &str =
    "android.hardware.security.see.authmgr.IAuthMgrAuthorization/default";
const DEVICE_STATE_SERVICE_NAME: &str =
    "android.hardware.security.see.devicestate.IDeviceState/default";

/// Get all supported VINTF services.
pub fn get_supported_vintf_services() -> &'static [&'static str] {
    &[
        SECURE_STORAGE_SERVICE_NAME,
        HWCRYPTO_KEY_SERVICE_NAME,
        AUTHMGR_SERVICE_NAME,
        DEVICE_STATE_SERVICE_NAME,
    ]
}

/// Get an SpIBinder for a given service name.
/// This function will block until the service is available (the trusty port is exposed).
/// TODO: b/432264083 -This is exposed for the VINTF TA to work around the lack of Rust RpcBinder
/// support for retrieving an SpIBinder during session setup.
pub fn get_vintf_binder(service_name: &str) -> binder::Result<SpIBinder> {
    match service_name {
        SECURE_STORAGE_SERVICE_NAME => {
            Ok(wait_for_interface::<dyn ISecureStorage>(service_name)?.as_binder())
        }
        HWCRYPTO_KEY_SERVICE_NAME => {
            Ok(wait_for_interface::<dyn IHwCryptoKey>(service_name)?.as_binder())
        }
        AUTHMGR_SERVICE_NAME => {
            Ok(wait_for_interface::<dyn IAuthMgrAuthorization>(service_name)?.as_binder())
        }
        DEVICE_STATE_SERVICE_NAME => {
            Ok(wait_for_interface::<dyn IDeviceState>(service_name)?.as_binder())
        }
        _ => Err(StatusCode::NAME_NOT_FOUND)?,
    }
}
