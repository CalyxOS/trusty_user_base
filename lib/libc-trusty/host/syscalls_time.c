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

#include <errno.h>
#include <time.h>
#include <trusty/time.h>
#include <trusty_syscalls.h>

#define NS_PER_SEC 1000000000

long _trusty_nanosleep(uint32_t clock_id, uint32_t flags, uint64_t sleep_time) {
    if (clock_id != CLOCK_BOOTTIME && clock_id != 0) {
        errno = EINVAL;
        return -1;
    }
    if (flags != 0) {
        errno = EINVAL;
        return -1;
    }
    struct timespec sleep_time_spec;
    sleep_time_spec.tv_sec = sleep_time / NS_PER_SEC;
    sleep_time_spec.tv_nsec = sleep_time % NS_PER_SEC;
    return clock_nanosleep(CLOCK_BOOTTIME, flags, &sleep_time_spec, NULL);
}

long _trusty_gettime(uint32_t clock_id, uint32_t flags, int64_t* time) {
    if (clock_id != CLOCK_BOOTTIME && clock_id != 0) {
        errno = EINVAL;
        return -1;
    }
    if (flags != 0) {
        errno = EINVAL;
        return -1;
    }
    struct timespec ts;
    if (clock_gettime(CLOCK_BOOTTIME, &ts) != 0) {
        return -1;
    }
    *time = ts.tv_sec * NS_PER_SEC + ts.tv_nsec;
    return 0;
}
