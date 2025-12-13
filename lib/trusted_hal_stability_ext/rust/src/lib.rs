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

//! Rust library for adding required trusted HAL extensions.
//!
//! Android VTS tests require a way to ensure that TEE environments
//! are using a libbinder built with the correct default stability. This library can be
//! used to add the required extension to a service implementation to ensure that
//! the VTS tests can pass.

use android_hardware_security_see_ext::aidl::android::hardware::security::see::ext::ITrustedHalExt::{BnTrustedHalExt, ITrustedHalExt};
use android_hardware_security_see_ext::binder::{self, binder_impl::{Binder, Remotable}, Interface, Strong, FromIBinder};

/// Implementation of the ITrustedHalExt interface.
pub struct TrustedHalExt;

impl Interface for TrustedHalExt {}

impl ITrustedHalExt for TrustedHalExt {}

/// Adds an implementation of ITrustedHalExt as an extension to the given service.
pub fn add_trusted_hal_extension<I: FromIBinder + ?Sized, T: Remotable>(
    service: &Strong<I>,
) -> binder::Result<()> {
    let ext_service = TrustedHalExt;
    let ext_binder = BnTrustedHalExt::new_binder(ext_service, binder::BinderFeatures::default());

    let service_ibinder = service.as_binder().clone();
    let mut binder_native: Binder<T> = service_ibinder.try_into()?;
    binder_native.set_extension(&mut ext_binder.as_binder())?;
    Ok(())
}
