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
#pragma once

#include <vector>

#include <android/trusty/membuf/BnMemoryBufferContext.h>
#include <android/trusty/membuf/BnMemoryBufferShareVm.h>
#include <binder/RpcServerTrusty.h>
#include <lib/tipc/tipc.h>  // For handle_t

#define MEM_SHARE_PORT "android.trusty.membuf.IMemoryBufferShareVm/default"

handle_t deserialize_token(const std::vector<uint8_t>& serialized_token);

class MemoryBufferContext
    : public android::trusty::membuf::BnMemoryBufferContext {
public:
    static android::sp<MemoryBufferContext> Make(handle_t handle,
                                                 uint64_t ipa);

    MemoryBufferContext(handle_t handle, uint64_t ipa);

    ~MemoryBufferContext();

    android::binder::Status getPortableToken(
            android::trusty::membuf::MemoryBufferToken* aidl_return) override;
    android::binder::Status releaseMemoryBufferContext(
            int32_t* aidl_return) override;

private:
    handle_t handle_;
    bool destroyed_;
    uint64_t ipa_;
};

class MemoryBufferShare
    : public android::trusty::membuf::BnMemoryBufferShareVm {
public:
    static android::sp<MemoryBufferShare> Make(
            android::sp<android::RpcServerTrusty> srv);

    MemoryBufferShare();

    android::binder::Status shareMemoryBuffer(
            int64_t ipa,
            int32_t sizeBytes,
            android::trusty::membuf::ShareMemoryBufferResult* aidl_return)
            override;
};

android::sp<MemoryBufferShare> create_memory_buffer_share(
        android::sp<android::RpcServerTrusty> rpc_server);
