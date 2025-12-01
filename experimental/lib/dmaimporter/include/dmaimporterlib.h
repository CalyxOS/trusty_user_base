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

#ifdef __cplusplus
extern "C" {
#endif

/*
 * create_dma_object() - creates a reference to a DMA object shared by the host
 * @ipa: intermediate physical address where the object was mapped
 * @len: length in bytes of the object. Has to be a multiple of a page/granule
 * @handle: output. On success will contain a handle to the shared object
 *
 * Creates a memory reference object given the intermediate physical address and
 * length in bytes where the hypervisor has mapped an object provided by the
 * host. It will return a handle to this object that can be used by the caller
 * to map it. The service knows by the ipa provided which type of object has
 * been shared.
 *
 * Return: NO_ERROR on success, error code less than 0 on error.
 */
int create_dma_object(uint64_t ipa, uint64_t len, handle_t* handle);

/*
 * release_dma_object() - free a memory reference created by create_dma_object
 * @ipa: intermediate physical address used to create the object
 * @handle: handle to free
 *
 * Free a reference to a DMA object created previously by a call to
 * create_dma_object. @ipa is the address provided to the call to
 * create_dma_object and will be used to search for the handle to free.
 *
 * Return: NO_ERROR on success, error code less than 0 on error.
 */
int release_dma_object(handle_t* handle, uint64_t ipa);

#ifdef __cplusplus
}
#endif