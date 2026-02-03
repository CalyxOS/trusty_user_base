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

/**
 * @file interface/trusty_cli/trusty_cli.h
 * @brief Contains types, enums and structs that should be commonly used by
 * trusty cli service and related android application.
 */

#include <stddef.h>
#include <stdint.h>

#define TRUSTY_CLI_PORT "com.android.trusty.trusty_cli"

/**
 * Command to execute.
 *
 * @TRUSTY_CLI_CMD_EXEC_CONSOLE: execute specific Trusty LK console command.
 */
typedef enum trusty_cli_cmd : uint32_t {
    TRUSTY_CLI_CMD_EXEC_CONSOLE = 0,
} trusty_cli_cmd_id_t;

/**
 * trusty cli response code
 *
 * @TRUSTY_CLI_STATUS_SUCCESS:         trusty cli command succeeded.
 * @TRUSTY_CLI_STATUS_REQUEST:         special value to use with request
 * messages.
 * @TRUSTY_CLI_STATUS_INTERIM:         Interim (frame) message with some
 * payload. More messages following up are expected.
 * @TRUSTY_CLI_STATUS_INVALID_REQUEST: Arguments don't validate.
 * @TRUSTY_CLI_STATUS_FORBIDDEN:       Process requesting cmd execution does not
 * have access to trusty cli.
 * @TRUSTY_CLI_STATUS_INTERNAL_ERROR:  Failed to exec a command. Failure reason
 * might be sent in a follow-up frame
 * @TRUSTY_CLI_STATUS_NOT_IMPLEMENTED: Requested command is not yet implemented
 * by the service.
 */
typedef enum trusty_cli_status : uint32_t {
    TRUSTY_CLI_STATUS_SUCCESS = 0,
    TRUSTY_CLI_STATUS_REQUEST = 0,
    TRUSTY_CLI_STATUS_INTERIM = 1,
    TRUSTY_CLI_STATUS_INVALID_REQUEST = 2,
    TRUSTY_CLI_STATUS_FORBIDDEN = 3,
    TRUSTY_CLI_STATUS_INTERNAL_ERROR = 4,
    TRUSTY_CLI_STATUS_NOT_IMPLEMENTED = 5,
} trusty_cli_status_t;

/**
 * @brief Output modes supported by trusty cli
 * @TRUSTY_CLI_OUTPUT_MODE_SEND_EACH send each message as soon as data was
 * passed to trusty_cli_printf;
 * @TRUSTY_CLI_OUTPUT_MODE_BUFFERIZE print data passed to trusty_cli_printf to
 * an inner buffer to be sent later;
 */
enum trusty_cli_output_mode {
    TRUSTY_CLI_OUTPUT_MODE_SEND_EACH = 0,
    TRUSTY_CLI_OUTPUT_MODE_BUFFERIZE = 1,
};

/**
 * trusty cli ipc message header.
 * @cmd: command id from @trusty_cli_cmd enum.
 * @status: status of the command execution from @trusty_cli_status; should be
 * set to 0 for requests.
 * @payload_size: size of buffer to follow (if any)
 * @reserved: a field to align struct size to 64 bit, reserved for future use
 */
struct trusty_cli_message_header {
    uint32_t cmd;
    uint32_t status;
    uint32_t payload_size;
    uint32_t reserved;
};

/**
 * @TRUSTY_CLI_MAX_SIZE - Maximum size of data to be transmitted by the service
 * at a time.
 * @TRUSTY_CLI_REQUEST_PAYLOAD_SIZE - max size of request payload.
 * @TRUSTY_CLI_DEFAULT_PRINT_BUFFER_SIZE - Buffer size used for data
 * bufferization (one page).
 */
#define TRUSTY_CLI_MAX_SIZE 2048
#define TRUSTY_CLI_REQUEST_PAYLOAD_SIZE 256
#define TRUSTY_CLI_DEFAULT_PRINT_BUFFER_SIZE 4096
