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

//! Stub library to get build working.
//!
//! Only for build and testing. Production build shall link with a different library
//! which provides a valid platform security patch timestamp, such as
//! `platform_security_patch_timestamp_rs` in `packages/modules/Virtualization/guest/pvmfw/Android.bp`.

pub const TIMESTAMP: u64 = 0;
