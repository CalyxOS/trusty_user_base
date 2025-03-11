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

//! A wrapper around the `RawHandleSet`, that is aware of the handle wrapper type stored in the
//! raw handle set. The wrapper also enforces the limits on ports and connections per TA.
use crate::raw::{service_handle::PortWrapper, HandleType, RawHandleSet, ServiceHandle};
use crate::{
    ConnectResult, Handle, MessageResult, PortCfg, Result, TipcError, UnbufferedService, Uuid,
};
use alloc::sync::Arc;
use log::error;
use std::sync::Mutex;
use trusty_std::TryClone;

/// A wrapper around the `RawHandleSet` exposed to the TAs and the event loop runner.
pub struct HandleSetWrapper<S: UnbufferedService> {
    handle_set: Mutex<RawHandleSet<ServiceHandle<S>>>,
}

impl<S: UnbufferedService> HandleSetWrapper<S> {
    pub fn new() -> Result<Self> {
        Ok(Self { handle_set: Mutex::new(RawHandleSet::<ServiceHandle<S>>::new()?) })
    }

    /// Waits on the raw handle set and returns the event cookie object and the event.
    pub fn wait(&self, timeout: Option<u32>) -> Result<(Arc<ServiceHandle<S>>, u32)> {
        self.handle_work_queue()?;
        let hs = self.handle_set.lock().unwrap();
        hs.wait(timeout)
    }

    /// Add a port type connection to the handle set wrapper
    pub fn add_port(&self, cfg: &PortCfg, service: Arc<S>) -> Result<PortWrapper<S>> {
        // SAFETY: syscall, config path is borrowed and outlives the call.
        // Return value is either a negative error code or a valid handle.
        let rc = unsafe {
            trusty_sys::port_create(
                cfg.get_path().as_ptr(),
                cfg.get_msg_queue_len(),
                cfg.get_msg_max_size(),
                cfg.get_flags(),
            )
        };
        if rc < 0 {
            Err(TipcError::from_uapi(rc))
        } else {
            let raw_handle_fd = rc as i32;
            let handle = Handle::from_raw(raw_handle_fd)?;
            let service_handle =
                Arc::new(ServiceHandle::<S>::new_port_wrapper(cfg.try_clone()?, handle, service));
            let hs = self.handle_set.lock().unwrap();
            hs.register(service_handle.clone())?;
            Ok(PortWrapper::new(self, service_handle))
        }
    }

    /// Dispatch an event on a port handle
    pub fn handle_connect(&self, service_handle: Arc<ServiceHandle<S>>) -> Result<()> {
        if let HandleType::Port(cfg) = &service_handle.ty {
            let mut peer = Uuid::from_bytes(&[0; Uuid::UUID_BYTE_LEN]);
            // SAFETY: syscall. The port owns its handle, so it is still valid as
            // a raw fd. The peer structure outlives this call and is mutably
            // borrowed by the call to initialize the structure's data.
            let rc = unsafe {
                trusty_sys::accept(service_handle.handle.as_raw_fd(), peer.as_mut_ptr()) as i32
            };
            let connection_handle = Handle::from_raw(rc)?;

            // Check against access control list if we were given one
            if let Some(uuids) = cfg.get_uuid_allow_list() {
                if !uuids.contains(&peer) {
                    error!("UUID {peer:?} isn't supported.\n");
                    return Err(TipcError::SystemError(trusty_sys::Error::NotAllowed));
                }
            }
            let connect_result =
                service_handle.service.on_connect(&cfg, &connection_handle, &peer)?;
            if let ConnectResult::Accept(conn) = connect_result {
                self.add_connection(conn, connection_handle, Arc::clone(&service_handle.service))
                    .map(|_| ())
            } else {
                error!("Connection closed");
                return Err(TipcError::ChannelClosed);
            }
        } else {
            error!("A port type handle is expected. Received: {:?}", &service_handle.ty);
            return Err(TipcError::InvalidData);
        }
    }

    /// Register a connection handle in the handle set
    pub fn add_connection(
        &self,
        connection: S::Connection,
        handle: Handle,
        service: Arc<S>,
    ) -> Result<()> {
        let service_handle =
            ServiceHandle::<S>::new_connection_wrapper(handle, service, connection);
        let hs = self.handle_set.lock().unwrap();
        hs.register(Arc::new(service_handle))
    }

    /// Dispatch an event on a port handle
    pub fn handle_message(&self, service_handle: Arc<ServiceHandle<S>>) -> Result<MessageResult> {
        if let HandleType::Connection(conn) = &service_handle.ty {
            service_handle.service.on_message(conn, &service_handle.handle, &mut [])
        } else {
            error!("A connection type handle is expected. Received: {:?}", &service_handle.ty);
            return Err(TipcError::InvalidData);
        }
    }

    /// Dispatch a connection close event
    pub fn handle_disconnect(&self, service_handle: Arc<ServiceHandle<S>>) -> Result<()> {
        if let HandleType::Connection(conn) = &service_handle.ty {
            service_handle.service.on_disconnect(conn)
        } else {
            error!("A connection type handle is expected. Received: {:?}", &service_handle.ty);
            return Err(TipcError::InvalidData);
        }
        self.remove(service_handle)?;
        Ok(())
    }

    /// Remove a handle from the handle set
    pub fn remove(&self, service_handle: Arc<ServiceHandle<S>>) -> Result<()> {
        let hs = self.handle_set.lock().unwrap();
        hs.remove(service_handle)
    }

    /// Remove a handle from the handle set
    pub fn remove_raw(&self, handle: i32) -> Result<Arc<ServiceHandle<S>>> {
        let hs = self.handle_set.lock().unwrap();
        hs.remove_raw(handle)
    }
}
