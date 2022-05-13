/*
 * Copyright (C) 2021 The Android Open Source Project
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

use alloc::rc::{Rc, Weak};
use core::fmt;
use core::mem::{ManuallyDrop, MaybeUninit};
use log::error;
use trusty_std::alloc::{AllocError, Vec};
use trusty_std::ffi::CString;
use trusty_std::TryClone;
use trusty_sys::c_void;

use crate::handle::MAX_MSG_HANDLES;
use crate::sys;
use crate::{Deserialize, Handle, Result, TipcError};
use handle_set::HandleSet;

mod handle_set;

/// A description of a server-side IPC port.
///
/// A port configuration specifies the service port and various parameters for
/// the service. This configuration struct is a builder to set these parameters.
///
/// # Examples
///
/// ```
/// # impl Service for () {
/// #     type Connection = ();
/// #     type Message = ();
///
/// #     fn on_connect(
/// #         &self,
/// #         _port: &PortCfg,
/// #         _handle: &Handle,
/// #         _peer: &Uuid,
/// #     ) -> Result<Option<Self::Connection>> {
/// #         Ok(Some(()))
/// #     }
/// #
/// #     fn on_message(
/// #         &self,
/// #         _connection: &Self::Connection,
/// #         _handle: &Handle,
/// #         _msg: Self::Message,
/// #     ) -> Result<bool> {
/// #         Ok(true)
/// #     }
/// # }
///
/// let cfg = PortCfg::new("com.android.trusty.rust_port_test")
///     .msg_queue_len(4)
///     .msg_max_size(4096)
///     .allow_ta_connect();
///
/// let service = ();
/// let buffer = [0u8; 4096];
/// let manager = Manager::new(service, &[cfg], None, buffer);
/// ```
#[derive(Debug)]
pub struct PortCfg {
    path: CString,
    msg_queue_len: u32,
    msg_max_size: u32,
    flags: u32,
}

impl PortCfg {
    /// Construct a new port configuration for the given path
    pub fn new<T: AsRef<str>>(path: T) -> Result<Self> {
        Ok(Self {
            path: CString::try_new(path.as_ref())?,
            msg_queue_len: 1,
            msg_max_size: 4096,
            flags: 0,
        })
    }

    /// Construct a new port configuration for the given path
    ///
    /// This version takes ownership of the path and does not allocate.
    pub fn new_raw(path: CString) -> Self {
        Self { path, msg_queue_len: 1, msg_max_size: 4096, flags: 0 }
    }

    /// Set the message queue length for this port configuration
    pub fn msg_queue_len(self, msg_queue_len: u32) -> Self {
        Self { msg_queue_len, ..self }
    }

    /// Set the message maximum length for this port configuration
    pub fn msg_max_size(self, msg_max_size: u32) -> Self {
        Self { msg_max_size, ..self }
    }

    /// Allow connections from non-secure (Android) clients for this port
    /// configuration
    pub fn allow_ns_connect(self) -> Self {
        Self { flags: self.flags | sys::IPC_PORT_ALLOW_NS_CONNECT as u32, ..self }
    }

    /// Allow connections from secure (Trusty) client for this port
    /// configuration
    pub fn allow_ta_connect(self) -> Self {
        Self { flags: self.flags | sys::IPC_PORT_ALLOW_TA_CONNECT as u32, ..self }
    }
}

impl TryClone for PortCfg {
    type Error = AllocError;

    fn try_clone(&self) -> core::result::Result<Self, Self::Error> {
        Ok(Self { path: self.path.try_clone()?, ..*self })
    }
}

pub(crate) struct Channel<D: Dispatcher> {
    handle: Handle,
    ty: ChannelTy<D>,
}

impl<D: Dispatcher> PartialEq for Channel<D> {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl<D: Dispatcher> Eq for Channel<D> {}

impl<D: Dispatcher> fmt::Debug for Channel<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Channel {{")?;
        writeln!(f, "  handle: {:?},", self.handle)?;
        writeln!(f, "  ty: {:?},", self.ty)?;
        write!(f, "}}")
    }
}

enum ChannelTy<D: Dispatcher> {
    /// Service port with a configuration describing the port
    Port(PortCfg),

    /// Client connection
    Connection(D::Connection),
}

impl<D: Dispatcher> fmt::Debug for ChannelTy<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChannelTy::Port(cfg) => write!(f, "ChannelTy::Port({:?})", cfg),
            ChannelTy::Connection(_) => write!(f, "ChannelTy::Connection"),
        }
    }
}

impl<D: Dispatcher> Channel<D> {
    pub fn handle(&self) -> &Handle {
        &self.handle
    }

    pub fn is_port(&self) -> bool {
        match self.ty {
            ChannelTy::Port(..) => true,
            _ => false,
        }
    }

    pub fn is_connection(&self) -> bool {
        match self.ty {
            ChannelTy::Connection(..) => true,
            _ => false,
        }
    }

    /// Reconstruct a reference to this type from an opaque pointer.
    ///
    /// SAFETY: The opaque pointer must have been constructed using
    /// Weak::into_raw, which happens in HandleSet::do_set_ctrl to create a
    /// connection cookie.
    unsafe fn from_opaque_ptr<'a>(ptr: *const c_void) -> Option<Rc<Self>> {
        if ptr.is_null() {
            None
        } else {
            // We must not drop the weak pointer here, because we are not
            // actually taking ownership of it.
            let weak = ManuallyDrop::new(Weak::from_raw(ptr.cast()));
            weak.upgrade()
        }
    }

    pub(crate) fn try_new_port(cfg: &PortCfg) -> Result<Rc<Self>> {
        // SAFETY: syscall, config path is borrowed and outlives the call.
        // Return value is either a negative error code or a valid handle.
        let rc = unsafe {
            trusty_sys::port_create(
                cfg.path.as_ptr(),
                cfg.msg_queue_len,
                cfg.msg_max_size,
                cfg.flags,
            )
        };
        if rc < 0 {
            Err(TipcError::from_uapi(rc))
        } else {
            Ok(Rc::try_new(Self {
                handle: Handle::from_raw(rc as i32)?,
                ty: ChannelTy::Port(cfg.try_clone()?),
            })?)
        }
    }

    fn try_new_connection(handle: Handle, data: D::Connection) -> Result<Rc<Self>> {
        Ok(Rc::try_new(Self { handle, ty: ChannelTy::Connection(data) })?)
    }
}

/// Trusty APP UUID
#[derive(Clone, Eq, PartialEq)]
pub struct Uuid(trusty_sys::uuid);

/// A service which handles IPC messages for a collection of [`Port`]s.
///
/// A service which implements this interface can register itself, along with a
/// set of [`Port`]s it handles, with a [`Manager`] which then dispatches
/// connection and message events to this service.
pub trait Service {
    /// Generic type to association with a connection. `on_connect()` should
    /// create this type for a successful connection.
    type Connection;

    /// Type of message this service can receive.
    type Message: Deserialize;

    /// Called when a client connects
    ///
    /// Returns either `Ok(Some(Connection))` if the connection should be
    /// accepted or `Ok(None)` if the connection should be closed.
    fn on_connect(
        &self,
        port: &PortCfg,
        handle: &Handle,
        peer: &Uuid,
    ) -> Result<Option<Self::Connection>>;

    /// Called when the service receives a message.
    ///
    /// The service manager handles deserializing the message, which is then
    /// passed to this callback.
    ///
    /// Should return `Ok(true)` if the connection should be kept open. The
    /// connection will be closed if `Ok(false)` or `Err(_)` is returned.
    fn on_message(
        &self,
        connection: &Self::Connection,
        handle: &Handle,
        msg: Self::Message,
    ) -> Result<bool>;

    /// Called when the client closes a connection.
    fn on_disconnect(&self, _connection: &Self::Connection) {}
}

pub trait Dispatcher {
    /// Generic type to association with a connection. `on_connect()` should
    /// create this type for a successful connection.
    type Connection;

    /// Called when a client connects
    ///
    /// Returns either `Ok(Some(Connection))` if the connection should be
    /// accepted or `Ok(None)` if the connection should be closed.
    fn on_connect(
        &self,
        port: &PortCfg,
        handle: &Handle,
        peer: &Uuid,
    ) -> Result<Option<Self::Connection>>;

    /// Called when the service receives a message.
    ///
    /// The service manager handles deserializing the message, which is then
    /// passed to this callback.
    ///
    /// Should return `Ok(true)` if the connection should be kept open. The
    /// connection will be closed if `Ok(false)` or `Err(_)` is returned.
    fn on_message(
        &self,
        connection: &Self::Connection,
        handle: &Handle,
        msg: &[u8],
        msg_handles: &[Handle],
    ) -> Result<bool>;

    /// Called when the client closes a connection.
    fn on_disconnect(&self, _connection: &Self::Connection) {}

    /// Get the maximum possible length of any message handled by this
    /// dispatcher.
    fn max_message_length(&self) -> usize;
}

// Implementation of a static dispatcher for services with only a single message
// type.
impl<T: Service> Dispatcher for T {
    type Connection = T::Connection;

    fn on_connect(
        &self,
        port: &PortCfg,
        handle: &Handle,
        peer: &Uuid,
    ) -> Result<Option<Self::Connection>> {
        T::on_connect(self, port, handle, peer)
    }

    fn on_message(
        &self,
        connection: &Self::Connection,
        handle: &Handle,
        msg: &[u8],
        msg_handles: &[Handle],
    ) -> Result<bool> {
        let msg = T::Message::deserialize(msg, msg_handles).map_err(|e| {
            error!("Could not parse message: {:?}", e);
            TipcError::InvalidData
        })?;
        T::on_message(self, connection, handle, msg)
    }

    fn max_message_length(&self) -> usize {
        T::Message::MAX_SERIALIZED_SIZE
    }
}

/// A manager that handles the IPC event loop and dispatches connections and
/// messages to a generic service.
pub struct Manager<
    D: Dispatcher,
    B: AsMut<[u8]> + AsRef<[u8]>,
    const PORT_COUNT: usize,
    const MAX_CONNECTION_COUNT: usize,
> {
    dispatcher: D,
    handle_set: HandleSet<D, PORT_COUNT, MAX_CONNECTION_COUNT>,
    buffer: B,
}

impl<
        S: Service,
        B: AsMut<[u8]> + AsRef<[u8]>,
        const PORT_COUNT: usize,
        const MAX_CONNECTION_COUNT: usize,
    > Manager<S, B, PORT_COUNT, MAX_CONNECTION_COUNT>
{
    /// Create a new service manager for the given service and ports.
    ///
    /// The manager will receive data into the buffer `B`, so this buffer needs
    /// to be at least as large as the largest message this service can handle.
    ///
    /// # Examples
    ///
    /// ```
    /// struct MyService;
    ///
    /// impl Service for MyService {
    ///     type Connection = ();
    ///     type Message = ();
    ///
    ///     fn on_connect(
    ///         &self,
    ///         _port: &PortCfg,
    ///         _handle: &Handle,
    ///         _peer: &Uuid,
    ///     ) -> Result<Option<Self::Connection>> {
    ///         Ok(Some(()))
    ///     }
    ///
    ///     fn on_message(
    ///         &self,
    ///         _connection: &Self::Connection,
    ///         _handle: &Handle,
    ///         _msg: Self::Message,
    ///     ) -> Result<bool> {
    ///         Ok(true)
    ///     }
    /// }
    ///
    /// let service = MyService;
    /// let cfg = PortCfg::new("com.android.trusty.rust_port_test");
    /// let buffer = [0u8; 4096];
    /// let mut manager = Manager::<_, _, 1, 1>::new(service, &[cfg], buffer);
    ///
    /// manager.run_event_loop()
    ///     .expect("Service manager encountered an error");
    /// ```
    pub fn new(service: S, port_cfgs: &[PortCfg; PORT_COUNT], buffer: B) -> Result<Self> {
        if buffer.as_ref().len() < service.max_message_length() {
            return Err(TipcError::NotEnoughBuffer);
        }
        let ports: Vec<Rc<Channel<S>>> =
            port_cfgs.iter().map(Channel::try_new_port).collect::<Result<_>>()?;
        let ports: [Rc<Channel<S>>; PORT_COUNT] = ports
            .try_into()
            .expect("This is impossible. Array size must match expected PORT_COUNT");
        let handle_set = HandleSet::try_new(ports)?;

        Ok(Self { dispatcher: service, handle_set, buffer })
    }
}

impl<
        D: Dispatcher,
        B: AsMut<[u8]> + AsRef<[u8]>,
        const PORT_COUNT: usize,
        const MAX_CONNECTION_COUNT: usize,
    > Manager<D, B, PORT_COUNT, MAX_CONNECTION_COUNT>
{
    /// Run the service event loop.
    ///
    /// Only returns if an error occurs.
    pub fn run_event_loop(mut self) -> Result<()> {
        loop {
            let event = self.handle_set.wait(None)?;
            // SAFETY: This cookie was previously initialized by the handle set.
            // Its lifetime is managed by the handle set, so we are sure that
            // the cookie is still valid if the channel is still in this handle
            // set.
            let channel: Rc<Channel<D>> = unsafe { Channel::from_opaque_ptr(event.cookie) }
                .ok_or_else(|| {
                    // The opaque pointer associated with this handle could not
                    // be converted back into a `Channel`. This should never
                    // happen, but throw an internal error if it does.
                    error!("Connection cookie was invalid");
                    TipcError::InvalidData
                })?;
            self.handler(channel, &event)?;
        }
    }

    fn handler(&mut self, channel: Rc<Channel<D>>, event: &trusty_sys::uevent) -> Result<()> {
        // TODO: Abort on port errors?
        match &channel.ty {
            ChannelTy::Port(cfg) if event.event & (sys::IPC_HANDLE_POLL_READY as u32) != 0 => {
                self.handle_connect(&channel.handle, cfg)
            }
            ChannelTy::Connection(data) if event.event & (sys::IPC_HANDLE_POLL_MSG as u32) != 0 => {
                match self.handle_message(&channel.handle, &data) {
                    Ok(true) => Ok(()),
                    Ok(false) => {
                        self.handle_set.close(channel);
                        Ok(())
                    }
                    Err(e) => {
                        error!("Could not handle message, closing connection: {:?}", e);
                        self.handle_set.close(channel);
                        Ok(())
                    }
                }
            }
            ChannelTy::Connection(data) if event.event & (sys::IPC_HANDLE_POLL_HUP as u32) != 0 => {
                self.handle_disconnect(&channel.handle, &data);
                self.handle_set.close(channel);
                Ok(())
            }
            _ => {
                error!("Could not handle event {}", event.event);
                Err(TipcError::UnknownError)
            }
        }
    }

    fn handle_connect(&mut self, handle: &Handle, cfg: &PortCfg) -> Result<()> {
        let mut peer = MaybeUninit::zeroed();
        // SAFETY: syscall. The port owns its handle, so it is still valid as
        // a raw fd. The peer structure outlives this call and is mutably
        // borrowed by the call to initialize the structure's data.
        let rc = unsafe { trusty_sys::accept(handle.as_raw_fd(), peer.as_mut_ptr()) as i32 };
        let connection_handle = Handle::from_raw(rc)?;
        // SAFETY: accept did not return an error, so it has successfully
        // initialized the peer structure.
        let peer = unsafe { Uuid(peer.assume_init()) };

        // TODO: Implement access control

        let connection_data = self.dispatcher.on_connect(&cfg, &connection_handle, &peer)?;
        if let Some(data) = connection_data {
            let connection_channel = Channel::try_new_connection(connection_handle, data)?;
            self.handle_set.add_connection(connection_channel)?;
        }

        Ok(())
    }

    fn handle_message(&mut self, handle: &Handle, data: &D::Connection) -> Result<bool> {
        let mut handles: [Handle; MAX_MSG_HANDLES] = Default::default();
        let (byte_count, handle_count) =
            handle.recv_vectored(&mut [self.buffer.as_mut()], &mut handles)?;
        self.dispatcher.on_message(
            data,
            handle,
            &self.buffer.as_ref()[..byte_count],
            &handles[..handle_count],
        )
    }

    fn handle_disconnect(&mut self, _handle: &Handle, data: &D::Connection) {
        self.dispatcher.on_disconnect(data);
    }
}

#[cfg(test)]
mod test {
    use super::{Handle, PortCfg, Result, Service, Uuid};

    impl Service for () {
        type Connection = ();
        type Message = ();

        fn on_connect(
            &self,
            _port: &PortCfg,
            _handle: &Handle,
            _peer: &Uuid,
        ) -> Result<Option<Self::Connection>> {
            Ok(Some(()))
        }

        fn on_message(
            &self,
            _connection: &Self::Connection,
            _handle: &Handle,
            _msg: Self::Message,
        ) -> Result<bool> {
            Ok(true)
        }
    }
}
