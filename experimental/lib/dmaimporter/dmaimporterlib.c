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

#define TLOG_TAG "dmaimporterlib"

#include <interface/dmaimporter/dmaimporter.h>
#include <inttypes.h>
#include <lib/tipc/tipc.h>
#include <trusty_log.h>
#include <uapi/err.h>

static int dma_importer_call(ipc_msg_t* tx_msg, ipc_msg_t* rx_msg) {
    if ((tx_msg == NULL) || (rx_msg == NULL)) {
        TLOGI("dma_importer_call received a null pointer\n");
        return ERR_INVALID_ARGS;
    }
    struct dmaimporter_res* response;
    if ((rx_msg->num_iov < 1) || (rx_msg->iov[0].iov_base == NULL) ||
        (rx_msg->iov[0].iov_len != sizeof(struct dmaimporter_res))) {
        TLOGI("dma_importer_call received invalid rx_msg\n");
        return ERR_INVALID_ARGS;
    }
    TLOGI("Connecting to port %s\n", DMAIMPORTER_PORT);
    long rc = connect(DMAIMPORTER_PORT, IPC_CONNECT_WAIT_FOR_PORT);
    if (rc < 0) {
        TLOGE("error connecting to port %s: %ld\n", DMAIMPORTER_PORT, rc);
        return rc;
    }
    handle_t chan = (handle_t)rc;
    rc = send_msg(chan, tx_msg);
    if (rc < 0) {
        TLOGE("sending message to dma importer failed %ld\n", rc);
        goto err;
    }
    if (rc != sizeof(struct dmaimporter_req)) {
        TLOGE("wrong req_hdr size for dma importer received %ld\n", rc);
        goto err;
    }
    TLOGI("Waiting on channel\n");
    uevent_t uevt;
    rc = wait(chan, &uevt, INFINITE_TIME);
    if (rc != NO_ERROR) {
        TLOGE("wait for dma importer event failed %ld\n", rc);
        goto err;
    }
    ipc_msg_info_t inf;
    rc = get_msg(chan, &inf);
    if (rc != NO_ERROR) {
        TLOGE("error receiving event data for dma importer %ld\n", rc);
        goto err;
    }
    if (inf.len != sizeof(response)) {
        TLOGE("should have received %ld bytes of data, received %ld\n",
              sizeof(struct dmaimporter_res), inf.len);
        goto err;
    }
    rc = read_msg(chan, inf.id, 0, rx_msg);
    if (rc < 0) {
        TLOGE("error reading data for dma importer %ld\n", rc);
    }
    response = (struct dmaimporter_res*)rx_msg->iov[0].iov_base;
    rc = (int)response->status;
err:
    close(chan);
    return rc;
}

int create_dma_object(uint64_t ipa, uint64_t len, handle_t* handle) {
    if (handle == NULL) {
        TLOGE("Received a null handle pointerfor mapped memory object\n");
        return ERR_INVALID_ARGS;
    }
    handle_t rec_handles[1];
    struct dmaimporter_req req_hdr = {.cmd = MAP_IPA, .ipa = ipa, .len = len};
    struct iovec tx_iov = {
            .iov_base = &req_hdr,
            .iov_len = sizeof(req_hdr),
    };

    ipc_msg_t tx_msg = {
            .num_iov = 1,
            .iov = &tx_iov,
    };

    struct dmaimporter_res response;

    struct iovec rx_iov = {
            .iov_base = &response,
            .iov_len = sizeof(response),
    };
    ipc_msg_t rx_msg = {
            .num_iov = 1,
            .iov = &rx_iov,
            .num_handles = 1,
            .handles = rec_handles,
    };

    int rc = dma_importer_call(&tx_msg, &rx_msg);

    if ((rc == 0) && (rec_handles[0] > 0)) {
        *handle = rec_handles[0];
    }

    return rc;
}

int release_dma_object(handle_t* handle, uint64_t ipa) {
    if (handle == NULL) {
        TLOGE("Received a null handle pointer\n");
        return ERR_INVALID_ARGS;
    }
    struct dmaimporter_req req_hdr = {.cmd = RELEASE_IPA, .ipa = ipa};
    struct iovec tx_iov = {
            .iov_base = &req_hdr,
            .iov_len = sizeof(req_hdr),
    };

    ipc_msg_t tx_msg = {
            .num_iov = 1,
            .iov = &tx_iov,
            .num_handles = 1,
            .handles = handle,
    };

    struct dmaimporter_res response;

    struct iovec rx_iov = {
            .iov_base = &response,
            .iov_len = sizeof(response),
    };
    ipc_msg_t rx_msg = {
            .num_iov = 1,
            .iov = &rx_iov,
            .num_handles = 0,
            .handles = NULL,
    };

    return dma_importer_call(&tx_msg, &rx_msg);
}