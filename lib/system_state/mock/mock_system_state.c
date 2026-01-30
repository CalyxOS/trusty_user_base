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

#include <lib/system_state/system_state.h>
#include <stdlib.h>  /* For getenv and strtoul */
#include <uapi/err.h> /* For NO_ERROR */

static const char* system_state_flag_to_env_var_str(
        enum system_state_flag flag) {
    switch (flag) {
    case SYSTEM_STATE_FLAG_PROVISIONING_ALLOWED:
        return "TRUSTY_SYSTEM_STATE_FLAG_PROVISIONING_ALLOWED";
    case SYSTEM_STATE_FLAG_APP_LOADING_UNLOCKED:
        return "TRUSTY_SYSTEM_STATE_FLAG_APP_LOADING_UNLOCKED";
    case SYSTEM_STATE_FLAG_APP_LOADING_VERSION_CHECK:
        return "TRUSTY_SYSTEM_STATE_FLAG_APP_LOADING_VERSION_CHECK";
    }
}

static uint64_t system_state_flag_to_default_value(
        enum system_state_flag flag) {
    switch (flag) {
    case SYSTEM_STATE_FLAG_PROVISIONING_ALLOWED:
        return SYSTEM_STATE_FLAG_PROVISIONING_ALLOWED_VALUE_ALLOWED;
    case SYSTEM_STATE_FLAG_APP_LOADING_UNLOCKED:
        return 1;
    case SYSTEM_STATE_FLAG_APP_LOADING_VERSION_CHECK:
        return SYSTEM_STATE_FLAG_APP_LOADING_VERSION_CHECK_VALUE_REQUIRED;
    }
}

int system_state_get_flag(enum system_state_flag flag, uint64_t* valuep) {
    const char* env_var_str = system_state_flag_to_env_var_str(flag);
    uint64_t flag_value = system_state_flag_to_default_value(flag);

    const char* flag_value_str = getenv(env_var_str);
    if (flag_value_str) {
        flag_value = strtoul(flag_value_str, NULL, 0);
    }

    *valuep = flag_value;

    return NO_ERROR;
}
