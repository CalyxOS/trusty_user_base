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

#include "memory_buffer_share.hpp"

#define TLOG_TAG "memory-buffer-share"

#include <dmaimporterlib.h>
#include <lk/err_ptr.h>
#include <trusty_log.h>
#include <iterator>
#include <vector>

using android::binder::Status;
using android::trusty::membuf::BnMemoryBufferContext;
using android::trusty::membuf::BnMemoryBufferShareVm;
using android::trusty::membuf::IMemoryBufferContext;
using android::trusty::membuf::IMemoryBufferShareVm;
using android::trusty::membuf::MemoryBufferToken;
using android::trusty::membuf::ShareMemoryBufferResult;
using std::vector;

handle_t deserialize_token(const vector<uint8_t>& serialized_token) {
    handle_t handle = 0;

    for (auto element = serialized_token.rbegin();
         element != serialized_token.rend(); ++element) {
        handle = handle << 8;
        handle |= *element;
    }
    return handle;
}

android::sp<MemoryBufferContext> MemoryBufferContext::Make(handle_t handle,
                                                           uint64_t ipa) {
    return android::sp<MemoryBufferContext>::make(handle, ipa);
}

MemoryBufferContext::MemoryBufferContext(handle_t handle, uint64_t ipa)
        : handle_(handle), destroyed_(false), ipa_(ipa) {}

MemoryBufferContext::~MemoryBufferContext() {
    if (!destroyed_) {
        TLOGE("MemoryBufferContext object destroyed before calling release\n");
        int res = release_dma_object(&handle_, ipa_);
        if (res != NO_ERROR) {
            TLOGE("couldn't release handle %x with ipa: 0x%lx\n", handle_,
                  ipa_);
        }
        handle_ = -1;
        destroyed_ = true;
    }
}

Status MemoryBufferContext::getPortableToken(MemoryBufferToken* aidl_return) {
    TLOGI("In getPortableToken\n");
    if (aidl_return == nullptr) {
        TLOGE("Received a null MemoryBufferToken\n");
        return Status::fromExceptionCode(Status::EX_NULL_POINTER,
                                         "Received a null MemoryBufferToken");
    }
    if (destroyed_) {
        // object is no longer usable
        TLOGE("MemoryBufferToken has already been destroyed\n");
        return Status::fromExceptionCode(
                Status::EX_ILLEGAL_STATE,
                "MemoryBufferToken has already been destroyed");
    }
    // TODO: check if we should provide tampering protection to the token
    vector<uint8_t> serialized_token;
    TLOGI("serializing handle %x\n", handle_);
    handle_t token = handle_;
    for (size_t i = 0; i < sizeof(handle_t); i++) {
        uint8_t element = token & 0xff;
        serialized_token.push_back(element);
        token = token >> 8;
    }
    aidl_return->token = std::move(serialized_token);
    return Status::ok();
}

Status MemoryBufferContext::releaseMemoryBufferContext(int32_t* aidl_return) {
    if (destroyed_) {
        // object is no longer usable
        TLOGE("MemoryBufferToken has already been destroyed\n");
        return Status::fromExceptionCode(
                Status::EX_ILLEGAL_STATE,
                "MemoryBufferToken has already been destroyed");
    }
    destroyed_ = true;
    int res = release_dma_object(&handle_, ipa_);
    if (res != NO_ERROR) {
        TLOGE("couldn't release handle %x\n", handle_);
        return Status::fromExceptionCode(
                Status::EX_ILLEGAL_STATE,
                "Couldn't release MemoryBufferToken handle");
    } else {
        TLOGI("Released handle %x with ipa: 0x%lx\n", handle_, ipa_);
        handle_ = -1;
    }
    if (aidl_return != nullptr) {
        // TODO: change returned value
        *aidl_return = res;
    }
    return Status::ok();
}

android::sp<MemoryBufferShare> MemoryBufferShare::Make(
        android::sp<android::RpcServerTrusty> srv) {
    auto self = android::sp<MemoryBufferShare>::make();
    srv->setRootObject(self);
    return self;
}

MemoryBufferShare::MemoryBufferShare() {}

Status MemoryBufferShare::shareMemoryBuffer(
        int64_t ipa,
        int32_t sizeBytes,
        ShareMemoryBufferResult* aidl_return) {
    TLOGI("In shareMemoryBuffer\n");
    if (aidl_return == nullptr) {
        TLOGE("reReceived a null ShareMemoryBufferResult\n");
        return Status::fromExceptionCode(
                Status::EX_NULL_POINTER,
                "Received a null ShareMemoryBufferResult");
    }
    handle_t handle;
    int rc = create_dma_object(ipa, sizeBytes, &handle);
    if (rc < 0) {
        TLOGE("Couldn't get memory handle from ipa: %d\n", rc);
        return Status::fromExceptionCode(Status::EX_ILLEGAL_STATE,
                                         "Couldn't get memory handle from ipa");
    }
    android::sp<MemoryBufferContext> mem_buffer_ctx =
            MemoryBufferContext::Make(handle, ipa);
    aidl_return->context = mem_buffer_ctx;
    return Status::ok();
}

android::sp<MemoryBufferShare> create_memory_buffer_share(
        android::sp<android::RpcServerTrusty> rpc_server) {
    if (rpc_server == nullptr) {
        return nullptr;
    }

    return MemoryBufferShare::Make(rpc_server);
}
