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

#include "trusty_cli.h"

#define ENUM_TO_STR_PREFIX_CASE(prefix, value_name) \
    case prefix##_##value_name:                     \
        return #value_name

static inline const char* trusty_cli_cmd_to_str(trusty_cli_cmd_id_t cmd) {
    switch (cmd) { ENUM_TO_STR_PREFIX_CASE(TRUSTY_CLI_CMD, EXEC_CONSOLE); }

    return "UNKNOWN";
}

static inline const char* trusty_cli_status_to_str(trusty_cli_status_t status) {
    switch (status) {
        ENUM_TO_STR_PREFIX_CASE(TRUSTY_CLI_STATUS, SUCCESS);
        ENUM_TO_STR_PREFIX_CASE(TRUSTY_CLI_STATUS, INTERIM);
        ENUM_TO_STR_PREFIX_CASE(TRUSTY_CLI_STATUS, INVALID_REQUEST);
        ENUM_TO_STR_PREFIX_CASE(TRUSTY_CLI_STATUS, FORBIDDEN);
        ENUM_TO_STR_PREFIX_CASE(TRUSTY_CLI_STATUS, INTERNAL_ERROR);
        ENUM_TO_STR_PREFIX_CASE(TRUSTY_CLI_STATUS, NOT_IMPLEMENTED);
    }

    return "UNKNOWN";
}