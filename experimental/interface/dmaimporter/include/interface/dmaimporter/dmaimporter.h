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

#include <inttypes.h>
#include <uapi/err.h>

#define DMAIMPORTER_PORT "com.android.trusty.kernel.dmaimporter"

#define MAX_DMAIMPORTER_MSG_SIZE 128

#define MAP_IPA 1
#define RELEASE_IPA 2

#define DMA_IMPORTER_UNRECOVERABLE_ERROR (ERR_USER_BASE - 100)
#define HYPERVISOR_OPERATION_ERROR (DMA_IMPORTER_UNRECOVERABLE_ERROR - 1)

/**
 * struct dmaimporter_req - Request structure for communicating with the dma
 * importer service
 * @cmd: cmd to be executed, could be MAP_IPA or RELEASE_IPA.
 * @reserved: reserved field.
 * @ipa: physical address of the memory to import for MAP_IPA calls. Unused on
 *       RELEASE_IPA calls.
 * @len: size in bytes of the emmory chink to import for MAP_IPA calls. Unused
 * on RELEASE_IPA calls.
 *
 */
struct dmaimporter_req {
    uint32_t cmd;
    uint32_t reserved;
    uint64_t ipa;
    uint64_t len;
};

struct dmaimporter_res {
    uint64_t status;
};
