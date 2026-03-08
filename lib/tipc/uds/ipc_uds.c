#define _GNU_SOURCE

/*
 * trusty_ipc.h defines accept(), connect() and wait() wrappers that are
 * incompatible with posix. We include the posix versions in this file, so
 * disable these wrappers.
 */
#define TRUSTY_AVOID_POSIX_CONFLICTS 1

#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/uio.h>
#include <sys/un.h>
#include <trusty_ipc.h>
#include <trusty_log.h>
#include <uapi/err.h>
#include <uapi/trusty_peer_id.h>
#include <unistd.h>

#include "handle_set_epoll.h"

#define TLOG_TAG "ipc-uds"

/* Max path length for UDS */
#define UDS_MAX_PATH_LEN (sizeof(((struct sockaddr_un*)0)->sun_path) - 1)

handle_t port_create(const char* path,
                     uint32_t queue_size,
                     uint32_t max_buffer_size,
                     uint32_t flags) {
    (void)max_buffer_size; /* Unused for now */
    (void)flags;           /* Unused for now */

    if (strlen(path) >= UDS_MAX_PATH_LEN) {
        TLOGE("UDS path too long: %s\n", path);
        return ERR_INVALID_ARGS;
    }

    int fd = socket(AF_UNIX, SOCK_SEQPACKET | SOCK_NONBLOCK, 0);
    if (fd < 0) {
        TLOGE("socket failed: %s\n", strerror(errno));
        return ERR_IO;
    }

    struct sockaddr_un addr;
    memset(&addr, 0, sizeof(addr));
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, path, sizeof(addr.sun_path));

    /* Unlink existing socket file if it exists */
    unlink(path);

    if (bind(fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        TLOGE("bind failed for %s: %s\n", path, strerror(errno));
        close(fd);
        return ERR_IO;
    }

    if (listen(fd, queue_size) < 0) {
        TLOGE("listen failed for %s: %s\n", path, strerror(errno));
        close(fd);
        return ERR_IO;
    }

    /*
     * max_buffer_size is not directly used for UDS, but we acknowledge it.
     * It might imply setting SO_RCVBUF/SO_SNDBUF, but for simplicity, we omit
     * for now.
     */

    return fd;
}

enum trusty_peer_id_kind_t {
    UDS_TRUSTY_PEER_ID_KIND_SECURITY_LABEL =
            TRUSTY_PEER_ID_KIND_FIRST_USERSPACE_AVAILABLE + 0x10000,
    UDS_TRUSTY_PEER_ID_KIND_UID_GID,
};

struct uds_trusty_peer_id_security_label {
    trusty_peer_id_kind_t kind;
    char label[];
};

struct uds_trusty_peer_id_uid_gid {
    trusty_peer_id_kind_t kind;
    uid_t uid;
    gid_t gid;
};

static inline struct uds_trusty_peer_id_uid_gid*
trusty_peer_id_size_to_writable_uds_trusty_peer_id_uid_gid(
        struct trusty_peer_id* peer_id,
        uint32_t* peer_id_size) {
    assert(*peer_id_size >= sizeof(struct uds_trusty_peer_id_uid_gid));
    *peer_id_size = sizeof(struct uds_trusty_peer_id_uid_gid);
    peer_id->kind = UDS_TRUSTY_PEER_ID_KIND_UID_GID;
    return (struct uds_trusty_peer_id_uid_gid*)peer_id;
}

static int get_peer_id_from_uds(int client_fd,
                                struct trusty_peer_id* peer_id,
                                uint32_t* peer_id_size_p) {
    socklen_t peer_sec_size =
            *peer_id_size_p - offsetof(struct trusty_peer_id, data);
    int ret = getsockopt(client_fd, SOL_SOCKET, SO_PEERSEC, peer_id->data,
                         &peer_sec_size);
    if (ret == 0) {
        peer_id->kind = UDS_TRUSTY_PEER_ID_KIND_SECURITY_LABEL;
        *peer_id_size_p = sizeof(struct uds_trusty_peer_id_security_label) +
                          peer_sec_size;
        return NO_ERROR;
    }
    if (ret < 0 && errno != ENOPROTOOPT) {
        TLOGE("getsockopt SO_PEERSEC failed with unexpected error: %s\n",
              strerror(errno));
        return ERR_IO;
    }
    /* Fallback to SO_PEERCRED uid/gid if SO_PEERSEC is not supported */
    struct ucred peer_cred;
    socklen_t peer_cred_size = sizeof(peer_cred);
    ret = getsockopt(client_fd, SOL_SOCKET, SO_PEERCRED, &peer_cred,
                     &peer_cred_size);
    if (ret < 0) {
        TLOGE("getsockopt SO_PEERCRED failed: %s\n", strerror(errno));
        return ERR_IO;
    }
    struct uds_trusty_peer_id_uid_gid* uid_gid =
            trusty_peer_id_size_to_writable_uds_trusty_peer_id_uid_gid(
                    peer_id, peer_id_size_p);
    uid_gid->uid = peer_cred.uid;
    uid_gid->gid = peer_cred.gid;
    return NO_ERROR;
}

int accept2(handle_t port_handle,
            struct trusty_peer_id* peer_id,
            uint32_t* peer_id_size_p) {
    int ret;

    if (*peer_id_size_p < sizeof(struct uds_trusty_peer_id_uid_gid)) {
        TLOGE("peer_id_size must be >= %zu\n",
              sizeof(struct uds_trusty_peer_id_uid_gid));
        return ERR_INVALID_ARGS;
    }

    int client_fd = accept(port_handle, NULL, NULL);
    TLOGD("accept returned: %d\n", client_fd);
    if (client_fd < 0) {
        if (errno == EWOULDBLOCK || errno == EAGAIN) {
            return ERR_NO_MSG; /* No pending connection */
        }
        TLOGE("accept failed: %s\n", strerror(errno));
        return ERR_IO;
    }

    /*
     * Set client socket to non-blocking as well, consistent with Trusty IPC
     * model
     */
    int flags = fcntl(client_fd, F_GETFL, 0);
    if (flags == -1) {
        TLOGE("fcntl F_GETFL failed: %s\n", strerror(errno));
        ret = ERR_IO;
        goto err_after_accept;
    }
    if (fcntl(client_fd, F_SETFL, flags | O_NONBLOCK) == -1) {
        TLOGE("fcntl F_SETFL O_NONBLOCK failed: %s\n", strerror(errno));
        ret = ERR_IO;
        goto err_after_accept;
    }

    if (peer_id) {
        ret = get_peer_id_from_uds(client_fd, peer_id, peer_id_size_p);
        if (ret != NO_ERROR) {
            goto err_after_accept;
        }
        TLOGD("accept peer_id: kind=%d, size=%d\n", peer_id->kind,
              *peer_id_size_p);
    }

    return client_fd;

err_after_accept:
    close(client_fd);
    return ret;
}

handle_t trusty_connect(const char* path, uint32_t flags) {
    if (strlen(path) >= UDS_MAX_PATH_LEN) {
        TLOGE("UDS path too long: %s\n", path);
        return ERR_INVALID_ARGS;
    }

    int fd = socket(AF_UNIX, SOCK_SEQPACKET | SOCK_NONBLOCK, 0);
    if (fd < 0) {
        TLOGE("socket failed: %s\n", strerror(errno));
        return ERR_IO;
    }

    struct sockaddr_un addr;
    memset(&addr, 0, sizeof(addr));
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, path, sizeof(addr.sun_path));

    int rc;
    useconds_t sleep_time = 100;
    int retry_count = 0;
    int retry_print_count = 10;
    while (true) {
        errno = 0;
        rc = connect(fd, (struct sockaddr*)&addr, sizeof(addr));
        if (rc == 0) {
            if (retry_count >= retry_print_count) {
                TLOGE("connect %d ready, retry_count %d\n", fd, retry_count);
            }
            return fd;
        }
        if (rc < 0 &&
            (errno != EAGAIN && !(flags & IPC_CONNECT_WAIT_FOR_PORT))) {
            TLOGE("connect failed for %s: %s, retry_count %d\n", path,
                  strerror(errno), retry_count);
            close(fd);
            return ERR_IO;
        }
        retry_count++;
        if (retry_count == retry_print_count) {
            TLOGE("connect returned %d, errno %d, %s, keep retrying...\n", rc,
                  errno, strerror(errno));
        }
        usleep(sleep_time);
        sleep_time = sleep_time * 2;
        if (sleep_time > 10000000) {
            sleep_time = 10000000;
        }
    }
}

int get_msg(handle_t chan_handle, ipc_msg_info_t* msg_info) {
    /*
     * For SOCK_SEQPACKET, we can use MSG_PEEK | MSG_TRUNC to get the size of
     * the next message
     */
    char unused_buf[1]; /* Need a buffer, even if 1 byte, for recvmsg */
    struct iovec iov = {.iov_base = unused_buf, .iov_len = sizeof(unused_buf)};
    struct msghdr msg = {
            .msg_iov = &iov,
            .msg_iovlen = 1,
            .msg_name = NULL,
            .msg_namelen = 0,
            .msg_control = NULL,
            .msg_controllen = 0,
            .msg_flags = 0,
    };

    ssize_t bytes_peeked = recvmsg(chan_handle, &msg, MSG_PEEK | MSG_TRUNC);

    if (bytes_peeked == 0) {
        /* Peer closed connection, handle this from EPOLLHUP instead */
        return ERR_NO_MSG;
    }
    if (bytes_peeked < 0) {
        if (errno == EWOULDBLOCK || errno == EAGAIN) {
            return ERR_NO_MSG; /* No message available */
        }
        TLOGE("recvmsg MSG_PEEK|MSG_TRUNC failed: %s\n", strerror(errno));
        return ERR_IO;
    }

    /* bytes_peeked now contains the actual length of the next message */
    msg_info->len = bytes_peeked;
    msg_info->id = 0; /* Fake message ID */
    msg_info->num_handles = 0;

    return NO_ERROR;
}

int put_msg(handle_t chan_handle, uint32_t msg_id) {
    /*
     * For UDS stream sockets, message acknowledgement is not explicit.
     * The message is considered "put" once read.
     * This function is a no-op in this UDS implementation.
     */
    (void)chan_handle;  // Unused
    (void)msg_id;       // Unused
    return NO_ERROR;
}

ssize_t read_msg(handle_t chan_handle,
                 uint32_t msg_id,
                 uint32_t offset,
                 ipc_msg_t* msg) {
    /*
     * msg_id and offset are not directly applicable to UDS streams,
     * as messages are read sequentially.
     */
    (void)msg_id;

    if (offset != 0) {
        TLOGE("offset != 0 is not supported for UDS\n");
        return ERR_INVALID_ARGS;
    }

    /*
     * For SOCK_SEQPACKET, readv will read a single message, preserving
     * boundaries.
     */
    ssize_t total_read = readv(chan_handle, msg->iov, msg->num_iov);

    if (total_read == 0) {
        return ERR_CHANNEL_CLOSED;  // Peer closed connection
    }
    if (total_read < 0) {
        if (errno == EWOULDBLOCK || errno == EAGAIN) {
            /*
             * This should ideally not happen if get_msg indicated a message is
             * available
             */
            return ERR_NO_MSG;
        }
        TLOGE("readv failed: %s\n", strerror(errno));
        return ERR_IO;
    }

    return total_read;
}

ssize_t send_msg(handle_t chan_handle, ipc_msg_t* msg) {
    size_t total_payload_len = 0;
    for (unsigned int i = 0; i < msg->num_iov; ++i) {
        total_payload_len += msg->iov[i].iov_len;
    }

    ssize_t bytes_written = writev(chan_handle, msg->iov, msg->num_iov);

    if (bytes_written < 0) {
        if (errno == EWOULDBLOCK || errno == EAGAIN) {
            return ERR_BUSY;  // Cannot send right now
        }
        TLOGE("writev failed: %s\n", strerror(errno));
        return ERR_IO;
    }

    /*
     * For SOCK_SEQPACKET, writev should write the entire message or fail.
     * Partial writes for a single message are generally not expected.
     */
    if ((size_t)bytes_written < total_payload_len) {
        TLOGE("partial write on seqpacket (%zd/%zu), this should not happen for a single message\n",
              bytes_written, total_payload_len);
        return ERR_BUSY;
    }

    return total_payload_len; /* Return payload length as per Trusty API */
}

int trusty_handle_wait(handle_t handle, uevent_t* event, uint32_t timeout_ms) {
    struct pollfd pfd;
    pfd.fd = handle;
    pfd.events = 0;

    /* If handle is an epoll fd, then handle it in trusty_epoll_handle_wait() */
    int ret = trusty_epoll_handle_wait(handle, event, timeout_ms);
    if (ret == 0 || ret != ERR_INVALID_ARGS) {
        return ret;
    }

    /*
     * event is an output only parameter so request all poll events that we can
     * translate to Trusty events.
     */

    pfd.events |= POLLIN;
    pfd.events |= POLLHUP;
    pfd.events |= POLLERR;

    /* TODO: Set POLLOUT if send_msg() has returned ERR_BUSY previously */

    int poll_timeout = (timeout_ms == INFINITE_TIME) ? -1 : (int)timeout_ms;

    int rc;
    do {
        rc = poll(&pfd, 1, poll_timeout);
    } while (rc == -1 && errno == EINTR);

    if (rc < 0) {
        TLOGE("poll failed: %s\n", strerror(errno));
        return ERR_IO;
    }
    if (rc == 0) {
        return ERR_TIMED_OUT;
    }

    /* Clear the input event flags and set based on poll results */
    event->event = IPC_HANDLE_POLL_NONE;
    if (pfd.revents & POLLIN) {
        int is_listening = 0;
        socklen_t is_listening_size = sizeof(is_listening);
        /*
         * POLLIN can correspond to IPC_HANDLE_POLL_MSG or
         * IPC_HANDLE_POLL_READY. The caller (ipc.c) will interpret this based
         * on the handle type. We set both here, as ipc.c's handle_port and
         * handle_channel check for these.
         */
        if (getsockopt(handle, SOL_SOCKET, SO_ACCEPTCONN, &is_listening,
                   &is_listening_size) < 0) {
            TLOGE("getsockopt SO_ACCEPTCONN failed: %s\n", strerror(errno));
        } else if (is_listening) {
            event->event |= IPC_HANDLE_POLL_READY;
        } else {
            event->event |= IPC_HANDLE_POLL_MSG;
        }
    }
    if (pfd.revents & POLLOUT) {
        event->event |= IPC_HANDLE_POLL_SEND_UNBLOCKED;
    }
    if (pfd.revents & POLLHUP) {
        event->event |= IPC_HANDLE_POLL_HUP;
    }
    if (pfd.revents & POLLERR) {
        event->event |= IPC_HANDLE_POLL_ERROR;
    }

    return NO_ERROR;
}

/*
 * The 'close' function is assumed to be the standard POSIX close(2) and is not
 * reimplemented here.
 */
