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

/*
 * trusty_ipc.h defines accept(), connect() and wait() wrappers that are
 * incompatible with posix. We include the posix versions in this file, so
 * disable these wrappers.
 */
#define TRUSTY_AVOID_POSIX_CONFLICTS 1

/* Set _GNU_SOURCE to 1 to enable TEMP_FAILURE_RETRY. */
#define _GNU_SOURCE 1

#include <errno.h>
#include <lib/tipc/tipc.h>
#include <lk/macros.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/epoll.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <trusty_ipc.h>
#include <trusty_log.h>
#include <uapi/err.h>
#include <unistd.h>

#include "handle_set_epoll.h"

#define TLOG_TAG "handle-set-epoll"

struct tipc_epoll_data {
    handle_t handle;
    struct tipc_event_handler* handler;
};

/**
 * trusty_to_epoll_events() - Helper to map Trusty IPC event flags to epoll
 * event flags.
 * @trusty_events: A bitmask of Trusty IPC_HANDLE_POLL_* flags.
 *
 * Returns the corresponding bitmask of epoll event flags.
 */
static uint32_t trusty_to_epoll_events(uint32_t trusty_events) {
    uint32_t epoll_events = 0;
    if (trusty_events & IPC_HANDLE_POLL_MSG) {
        epoll_events |= EPOLLIN;
    }
    if (trusty_events & IPC_HANDLE_POLL_HUP) {
        epoll_events |= EPOLLHUP;
    }
    if (trusty_events & IPC_HANDLE_POLL_ERROR) {
        epoll_events |= EPOLLERR;
    }
    /*
     * IPC_HANDLE_POLL_READY can mean different things (port ready for connect,
     * channel ready for read). For epoll, EPOLLIN generally covers "ready for
     * read" or "ready for accept". The ipc.c logic will differentiate based on
     * handle type.
     */
    if (trusty_events & IPC_HANDLE_POLL_READY) {
        epoll_events |= EPOLLIN;
    }
    /*
     * IPC_HANDLE_POLL_SEND_UNBLOCKED only gets sent if an outgoing message was
     * previousely rejected and there is now room again for another message.
     * EPOLLOUT is set whenever there is room. If a trusty app requests
     * IPC_HANDLE_POLL_SEND_UNBLOCKED is it not neccecarily safe to request
     * EPOLLOUT. Ignore this event for now since it is not used by every
     * app. Implementing this event on top of epoll will require wrapping all
     * send/write operations on this channel with a helper function that can
     * enable the EPOLLOUT event only when a message is rejected because the
     * channel is full.
     *
     * TODO: Implement support for this event.
     */
    return epoll_events;
}

/**
 * epoll_to_trusty_events() - Helper to map epoll event flags back to Trusty IPC
 * event flags.
 * @epoll_events: A bitmask of epoll event flags.
 * @handle:       The handle that triggered the event, used to differentiate
 *                between EPOLLIN for ports and channels.
 *
 * Returns the corresponding bitmask of Trusty IPC_HANDLE_POLL_* flags.
 */
static uint32_t epoll_to_trusty_events(uint32_t epoll_events,
                                       handle_t handle) {
    uint32_t trusty_events = 0;
    if (epoll_events & EPOLLIN) {
        int is_listening = 0;
        socklen_t is_listening_size = sizeof(is_listening);
        /*
         * EPOLLIN can correspond to IPC_HANDLE_POLL_MSG or
         * IPC_HANDLE_POLL_READY. The caller (ipc.c) will interpret this based
         * on the handle type. We set both here, as ipc.c's handle_port and
         * handle_channel check for these.
         */
        if (getsockopt(handle, SOL_SOCKET, SO_ACCEPTCONN, &is_listening,
                   &is_listening_size) < 0) {
            TLOGE("getsockopt SO_ACCEPTCONN failed: %s\n", strerror(errno));
        } else if (is_listening) {
            trusty_events |= IPC_HANDLE_POLL_READY;
        } else {
            trusty_events |= IPC_HANDLE_POLL_MSG;
        }
    }
    if (epoll_events & EPOLLHUP) {
        trusty_events |= IPC_HANDLE_POLL_HUP;
    }
    if (epoll_events & EPOLLERR) {
        trusty_events |= IPC_HANDLE_POLL_ERROR;
    }
    if (epoll_events & EPOLLOUT) {
        trusty_events |= IPC_HANDLE_POLL_SEND_UNBLOCKED;
    }
    return trusty_events;
}

/**
 * handle_set_create() - Create a new epoll-based handle set.
 *
 * Returns the epoll fd on success, or -1 on failure.
 */
int handle_set_create(void) {
    int epoll_fd = epoll_create1(0); /* 0 for default flags */
    if (epoll_fd < 0) {
        TLOGE("%s: Failed to create epoll fd: %s\n", __func__, strerror(errno));
        return -1;
    }
    return epoll_fd;
}

/**
 * handle_set_epoll_handle_add() - Add a handle to the epoll-based handle set.
 * @handle: The epoll fd.
 * @evt:    The uevent structure containing the handle to add and its event
 *          handler.
 *
 * Returns 0 on success, or an error code on failure.
 */
static int handle_set_epoll_handle_add(handle_t handle, struct uevent* evt) {
    int ret;
    struct tipc_epoll_data* data = malloc(sizeof(*data));
    if (!data) {
        return ERR_NO_MEMORY;
    }
    data->handle = evt->handle;
    data->handler = evt->cookie;
    struct epoll_event ev = {
            .events = trusty_to_epoll_events(evt->event),
            .data.ptr = data,
    };
    ret = epoll_ctl(handle, EPOLL_CTL_ADD, evt->handle, &ev);
    if (ret < 0) {
        free(data);
        TLOGE("%s: Failed to add handle %d to epoll: %s\n", __func__,
              evt->handle, strerror(errno));
        if (errno == EBADF || errno == EPERM)
            return ERR_INVALID_ARGS;
        if (errno == ENOMEM)
            return ERR_NO_MEMORY;
        if (errno == EEXIST)
            return ERR_ALREADY_EXISTS;
        return ERR_IO;
    }
    return 0;
}

/**
 * handle_set_epoll_handle_mod() - Modify a handle in the epoll-based handle
 * set.
 * @handle: The epoll fd.
 * @evt:    The uevent structure containing the handle to modify and its event
 *          handler.
 *
 * TODO: Implement this.
 *
 * Returns 0 on success, or an error code on failure.
 */
static int handle_set_epoll_handle_mod(handle_t handle, struct uevent* evt) {
    (void)handle;
    (void)evt;
    TLOGE("%s: not implemented\n", __func__);
    return ERR_NOT_IMPLEMENTED;
}

/**
 * handle_set_epoll_handle_del() - Remove a handle from the epoll-based handle
 * set.
 * @handle: The epoll fd.
 * @evt:    The uevent structure containing the handle to remove.
 *
 * Note that this implementation leaks the tipc_epoll_data structure that
 * was allocated in handle_set_epoll_handle_add() since epoll does not provide
 * a way to retrieve this pointer. Since this code is currently only used for
 * testing, implementing a leak-free version is left as a TODO.
 *
 * Returns 0 on success, or an error code on failure.
 */
static int handle_set_epoll_handle_del(handle_t handle, struct uevent* evt) {
    struct epoll_event ev;
    memset(&ev, 0, sizeof(ev));
    int ret = epoll_ctl(handle, EPOLL_CTL_DEL, evt->handle, &ev);
    if (ret < 0) {
        TLOGE("%s: Failed to delete handle %d from epoll: %s\n", __func__,
              evt->handle, strerror(errno));
        if (errno == EBADF || errno == ENOENT)
            return ERR_NOT_FOUND;
        return ERR_IO;
    }
    /*
     * TODO: Find and free tipc_epoll_data struct that was allocated in
     * handle_set_epoll_handle_add().
     */

     return 0;
}

/**
 * handle_set_ctrl() - Control the epoll-based handle set.
 * @handle: The epoll fd.
 * @cmd:    The command to execute.
 * @evt:    The uevent structure containing the handle to modify and its event
 *          handler.
 *
 * Returns 0 on success, or an error code on failure.
 */
int handle_set_ctrl(handle_t handle, uint32_t cmd, struct uevent* evt) {
    switch (cmd) {
        case HSET_ADD:
            return handle_set_epoll_handle_add(handle, evt);
        case HSET_DEL:
            return handle_set_epoll_handle_del(handle, evt);
        case HSET_MOD:
            return handle_set_epoll_handle_mod(handle, evt);
        default:
            return ERR_INVALID_ARGS;
    }
}

/**
 * trusty_epoll_handle_wait() - Wait for events on epoll-based handle set.
 * @handle:        The epoll fd.
 * @event:         The uevent structure to populate with the event.
 * @timeout_msecs: The timeout in milliseconds.
 *
 * Returns 0 on success, or an error code on failure. Specifically it returns
 * ERR_INVALID_ARGS if handle is not an epoll fd so that the caller can
 * fallback to a non-epoll implementation.
 */
int trusty_epoll_handle_wait(handle_t handle, struct uevent* event, uint32_t timeout_msecs) {
    const int max_events = 1;
    struct epoll_event epoll_events[max_events];
    int num_ready = TEMP_FAILURE_RETRY(
            epoll_wait(handle, epoll_events, max_events, timeout_msecs));
    if (num_ready < 0) {
        if (errno == EINVAL) {
            return ERR_INVALID_ARGS;
        }
        TLOGE("%s: epoll_wait failed: %s\n", __func__, strerror(errno));
        return ERR_IO;
    }
    if (num_ready == 0) {
        return ERR_TIMED_OUT;
    }
    struct epoll_event* ev = &epoll_events[0];
    struct tipc_epoll_data* data = ev->data.ptr;
    event->handle = data->handle;
    event->event = epoll_to_trusty_events(ev->events, data->handle);
    event->cookie = data->handler;

    return 0;
}
