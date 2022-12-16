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
/// let manager = Manager::new(service, cfg, buffer);
/// ```
#[derive(Debug, Eq, PartialEq)]
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
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Uuid(trusty_sys::uuid);

/// A service which handles IPC messages for a collection of ports.
///
/// A service which implements this interface can register itself, along with a
/// set of ports it handles, with a [`Manager`] which then dispatches
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
        msg_handles: &mut [Option<Handle>],
    ) -> Result<bool>;

    /// Called when the client closes a connection.
    fn on_disconnect(&self, _connection: &Self::Connection) {}

    /// Get the list of ports this dispatcher handles.
    fn port_configurations(&self) -> &[PortCfg];

    /// Get the maximum possible length of any message handled by this
    /// dispatcher.
    fn max_message_length(&self) -> usize;
}

// Implementation of a static dispatcher for services with only a single message
// type.
pub struct SingleDispatcher<S: Service> {
    service: S,
    ports: [PortCfg; 1],
}

impl<S: Service> SingleDispatcher<S> {
    fn new(service: S, port: PortCfg) -> Self {
        Self { service, ports: [port] }
    }
}

impl<S: Service> Dispatcher for SingleDispatcher<S> {
    type Connection = S::Connection;

    fn on_connect(
        &self,
        port: &PortCfg,
        handle: &Handle,
        peer: &Uuid,
    ) -> Result<Option<Self::Connection>> {
        self.service.on_connect(port, handle, peer)
    }

    fn on_message(
        &self,
        connection: &Self::Connection,
        handle: &Handle,
        msg: &[u8],
        msg_handles: &mut [Option<Handle>],
    ) -> Result<bool> {
        let msg = S::Message::deserialize(msg, msg_handles).map_err(|e| {
            error!("Could not parse message: {:?}", e);
            TipcError::InvalidData
        })?;
        self.service.on_message(connection, handle, msg)
    }

    fn on_disconnect(&self, connection: &Self::Connection) {
        self.service.on_disconnect(connection)
    }

    fn max_message_length(&self) -> usize {
        S::Message::MAX_SERIALIZED_SIZE
    }

    fn port_configurations(&self) -> &[PortCfg] {
        &self.ports
    }
}

/// Create a new service dispatcher that can handle a specified set of service
/// types.
///
/// This macro creates a multi-service dispatcher that holds different types of
/// services that should share the same event loop and dispatches to the
/// relevant service based on the connection port. Services must implement the
/// [`Service`] trait. An instance of this dispatcher must have a single const
/// usize parameter specifying how many ports the dispatcher will handle.
/// This macro has limited lifetime support. A single lifetime can be
/// used for the ServiceDispatcher enum and the included services (see the
/// supported definition in the Examples section).
///
/// # Examples
/// ```
/// service_dispatcher! {
///     enum ServiceDispatcher {
///         Service1,
///         Service2,
///     }
/// }
///
/// // Create a new dispatcher that handles two ports
/// let dispatcher = ServiceDispatcher::<2>::new()
///     .expect("Could not allocate service dispatcher");
///
/// let cfg = PortCfg::new(&"com.android.trusty.test_port1).unwrap();
/// dispatcher.add_service(Rc::new(Service1), cfg).expect("Could not add service 1");
///
/// let cfg = PortCfg::new(&"com.android.trusty.test_port2).unwrap();
/// dispatcher.add_service(Rc::new(Service2), cfg).expect("Could not add service 2");
/// ```
///
/// ## defining a dispatcher with multiple lifetimes
/// ```
/// service_dispatcher! {
///     enum ServiceDispatcher<'a> {
///         Service1<'a>,
///         Service2<'a>,
///     }
/// }
/// ```
#[macro_export]
macro_rules! service_dispatcher {
    (enum $name:ident $(<$elt: lifetime>)? {$($service:ident $(<$slt: lifetime>)? ),+ $(,)*}) => {
        /// Dispatcher that routes incoming messages to the correct server based on what
        /// port the message was sent to.
        ///
        /// This dispatcher adapts multiple different server types that expect different
        /// message formats for the same [`Manager`]. By using this dispatcher,
        /// different servers can be bound to different ports using the same event loop
        /// in the manager.
        struct $name<$($elt,)? const PORT_COUNT: usize> {
            // ports and services should always be kept in sync, i.e. the
            // service at index `i` services port `i`.
            ports: Vec<PortCfg>,
            services: Vec<ServiceKind$(<$elt>)?>,
        }

        impl<$($elt,)? const PORT_COUNT: usize> $name<$($elt,)? PORT_COUNT> {
            fn new() -> core::result::Result<Self, alloc::collections::TryReserveError> {
                use trusty_std::alloc::FallibleVec;
                Ok(Self {
                    ports: Vec::try_with_capacity(PORT_COUNT)?,
                    services: Vec::try_with_capacity(PORT_COUNT)?,
                })
            }

            fn add_service<T>(&mut self, service: Rc<T>, port: PortCfg) -> $crate::Result<()>
            where ServiceKind$(<$elt>)? : From<Rc<T>> {
                if self.ports.len() >= PORT_COUNT || self.services.len() >= PORT_COUNT {
                    return Err(TipcError::OutOfBounds);
                }
                // We unwrap here because we already checked capacity and we
                // don't want to allow ports and services to get out of sync,
                // e.g. the port was pushed but the service was not.
                self.ports.try_push(port).unwrap();
                self.services.try_push(service.into()).unwrap();
                Ok(())
            }
        }

        enum ServiceKind$(<$elt>)? {
            $($service(Rc<$service$(<$slt>)?>)),+
        }

        $(
            impl$(<$slt>)? From<Rc<$service$(<$slt>)?>> for ServiceKind$(<$slt>)? {
                fn from(service: Rc<$service$(<$slt>)?>) -> Self {
                    ServiceKind::$service(service)
                }
            }
        )+

        enum ConnectionKind$(<$elt>)?  {
            $($service(<$service$(<$slt>)? as $crate::Service>::Connection)),+
        }

        impl<$($elt,)? const PORT_COUNT: usize> $crate::Dispatcher for $name<$($elt,)? PORT_COUNT> {
            type Connection = (usize, ConnectionKind$(<$elt>)?);

            fn on_connect(
                &self,
                port: &PortCfg,
                handle: &Handle,
                peer: &Uuid,
            ) -> $crate::Result<Option<Self::Connection>> {
                let port_idx = self.ports.iter().position(|cfg| cfg == port)
                                                .ok_or(TipcError::InvalidPort)?;

                match &self.services[port_idx] {
                    $(ServiceKind::$service(s) => {
                        $crate::Service::on_connect(&**s, port, handle, peer)
                            .map(|c| c.map(|c| (port_idx, ConnectionKind::$service(c))))
                    })+
                }
            }

            fn on_message(
                &self,
                connection: &Self::Connection,
                handle: &Handle,
                msg: &[u8],
                msg_handles: &mut [Option<Handle>],
            ) -> $crate::Result<bool> {
                match &self.services[connection.0] {
                    $(ServiceKind::$service(s) => {
                        let msg = <$service as $crate::Service>::Message::deserialize(msg, msg_handles).map_err(|e| {
                            eprintln!("Could not parse message: {:?}", e);
                            TipcError::InvalidData
                        })?;
                        if let ConnectionKind::$service(conn) = &connection.1 {
                            $crate::Service::on_message(&**s, conn, handle, msg)
                        } else {
                            Err(TipcError::InvalidData)
                        }
                    })*
                }
            }

            fn on_disconnect(&self, connection: &Self::Connection) {
                match &self.services[connection.0] {
                    $(ServiceKind::$service(s) => {
                        if let ConnectionKind::$service(conn) = &connection.1 {
                            $crate::Service::on_disconnect(&**s, conn)
                        } else {
                            eprintln!("Expected a connection with kind {}", stringify!($service));
                        }
                    })*
                }
            }

            fn port_configurations(&self) -> &[PortCfg] {
                &self.ports
            }

            fn max_message_length(&self) -> usize {
                self.services.iter().map(|s| {
                    match s {
                        $(ServiceKind::$service(_) => {
                            <$service as $crate::Service>::Message::MAX_SERIALIZED_SIZE
                        })+
                    }
                }).max().unwrap_or(0usize)
            }
        }
    };

    (@make_none $service:ident) => { None };
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
    > Manager<SingleDispatcher<S>, B, PORT_COUNT, MAX_CONNECTION_COUNT>
{
    /// Create a new service manager for the given service and port.
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
    pub fn new(service: S, port_cfg: PortCfg, buffer: B) -> Result<Self> {
        let dispatcher = SingleDispatcher::new(service, port_cfg);
        Self::new_with_dispatcher(dispatcher, buffer)
    }
}

impl<
        D: Dispatcher,
        B: AsMut<[u8]> + AsRef<[u8]>,
        const PORT_COUNT: usize,
        const MAX_CONNECTION_COUNT: usize,
    > Manager<D, B, PORT_COUNT, MAX_CONNECTION_COUNT>
{
    /// Create a manager that can handle multiple services and ports
    ///
    /// A dispatcher handles mapping connections to services and parsing
    /// messages for the relevant service depending on which port the connection
    /// was made to. This allows multiple distinct services, each with their own
    /// message format and port to share the same event loop in the manager.
    ///
    /// See [`service_dispatcher!`] for details on how to create a dispatcher
    /// for use with this API.
    ///
    /// # Examples
    /// ```
    /// service_dispatcher! {
    ///     enum ServiceDispatcher {
    ///         Service1,
    ///         Service2,
    ///     }
    /// }
    ///
    /// // Create a new dispatcher that handles two ports
    /// let dispatcher = ServiceDispatcher::<2>::new()
    ///     .expect("Could not allocate service dispatcher");
    ///
    /// let cfg = PortCfg::new(&"com.android.trusty.test_port1).unwrap();
    /// dispatcher.add_service(Rc::new(Service1), cfg).expect("Could not add service 1");
    ///
    /// let cfg = PortCfg::new(&"com.android.trusty.test_port2).unwrap();
    /// dispatcher.add_service(Rc::new(Service2), cfg).expect("Could not add service 2");
    ///
    /// Manager::<_, _, 2, 4>::new_with_dispatcher(dispatcher, [0u8; 4096])
    ///     .expect("Could not create service manager")
    ///     .run_event_loop()
    ///     .expect("Service manager exited unexpectedly");
    /// ```
    pub fn new_with_dispatcher(dispatcher: D, buffer: B) -> Result<Self> {
        if buffer.as_ref().len() < dispatcher.max_message_length() {
            return Err(TipcError::NotEnoughBuffer);
        }

        let ports: Vec<Rc<Channel<D>>> = dispatcher
            .port_configurations()
            .iter()
            .map(Channel::try_new_port)
            .collect::<Result<_>>()?;
        let ports: [Rc<Channel<D>>; PORT_COUNT] = ports
            .try_into()
            .expect("This is impossible. Array size must match expected PORT_COUNT");
        let handle_set = HandleSet::try_new(ports)?;

        Ok(Self { dispatcher, handle_set, buffer })
    }

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
        let mut handles: [Option<Handle>; MAX_MSG_HANDLES] = Default::default();
        let (byte_count, handle_count) =
            handle.recv_vectored(&mut [self.buffer.as_mut()], &mut handles)?;
        self.dispatcher.on_message(
            data,
            handle,
            &self.buffer.as_ref()[..byte_count],
            &mut handles[..handle_count],
        )
    }

    fn handle_disconnect(&mut self, _handle: &Handle, data: &D::Connection) {
        self.dispatcher.on_disconnect(data);
    }
}

#[cfg(test)]
mod test {
    use super::{PortCfg, Service};
    use crate::handle::test::{first_free_handle_index, MAX_USER_HANDLES};
    use crate::{Deserialize, Handle, Manager, Result, Serialize, Serializer, TipcError, Uuid};
    use test::{expect, expect_eq};
    use trusty_std::alloc::FallibleVec;
    use trusty_std::ffi::CString;
    use trusty_std::format;
    use trusty_std::rc::Rc;
    use trusty_std::vec::Vec;
    use trusty_sys::Error;

    /// Maximum length of port path name
    const MAX_PORT_PATH_LEN: usize = 64;

    /// Maximum number of buffers per port
    const MAX_PORT_BUF_NUM: u32 = 64;

    /// Maximum size of port buffer
    const MAX_PORT_BUF_SIZE: u32 = 4096;

    const SRV_PATH_BASE: &str = "com.android.ipc-unittest";

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

    type Channel = super::Channel<super::SingleDispatcher<()>>;

    #[test]
    fn port_create_negative() {
        let path = [0u8; 0];

        expect_eq!(
            Channel::try_new_port(&PortCfg::new_raw(CString::try_new(&path[..]).unwrap())).err(),
            Some(TipcError::SystemError(Error::InvalidArgs)),
            "empty server path",
        );

        let mut path = format!("{}.port", SRV_PATH_BASE);

        let cfg = PortCfg::new(&path).unwrap().msg_queue_len(0);
        expect_eq!(
            Channel::try_new_port(&cfg).err(),
            Some(TipcError::SystemError(Error::InvalidArgs)),
            "no buffers",
        );

        let cfg = PortCfg::new(&path).unwrap().msg_max_size(0);
        expect_eq!(
            Channel::try_new_port(&cfg).err(),
            Some(TipcError::SystemError(Error::InvalidArgs)),
            "zero buffer size",
        );

        let cfg = PortCfg::new(&path).unwrap().msg_queue_len(MAX_PORT_BUF_NUM * 100);
        expect_eq!(
            Channel::try_new_port(&cfg).err(),
            Some(TipcError::SystemError(Error::InvalidArgs)),
            "large number of buffers",
        );

        let cfg = PortCfg::new(&path).unwrap().msg_max_size(MAX_PORT_BUF_SIZE * 100);
        expect_eq!(
            Channel::try_new_port(&cfg).err(),
            Some(TipcError::SystemError(Error::InvalidArgs)),
            "large buffers size",
        );

        while path.len() < MAX_PORT_PATH_LEN + 16 {
            path.push('a');
        }

        let cfg = PortCfg::new(&path).unwrap();
        expect_eq!(
            Channel::try_new_port(&cfg).err(),
            Some(TipcError::SystemError(Error::InvalidArgs)),
            "path is too long",
        );
    }

    #[test]
    fn port_create() {
        let mut channels: Vec<Rc<Channel>> = Vec::new();

        for i in first_free_handle_index()..MAX_USER_HANDLES - 1 {
            let path = format!("{}.port.{}{}", SRV_PATH_BASE, "test", i);
            let cfg = PortCfg::new(path).unwrap();
            let channel = Channel::try_new_port(&cfg);
            expect!(channel.is_ok(), "create ports");
            channels.try_push(channel.unwrap()).unwrap();

            expect_eq!(
                Channel::try_new_port(&cfg).err(),
                Some(TipcError::SystemError(Error::AlreadyExists)),
                "collide with existing port",
            );
        }

        // Creating one more port should succeed
        let path = format!("{}.port.{}{}", SRV_PATH_BASE, "test", MAX_USER_HANDLES - 1);
        let cfg = PortCfg::new(path).unwrap();
        let channel = Channel::try_new_port(&cfg);
        expect!(channel.is_ok(), "create ports");
        channels.try_push(channel.unwrap()).unwrap();

        // but creating colliding port should fail with different error code
        // because we actually exceeded max number of handles instead of
        // colliding with an existing path
        expect_eq!(
            Channel::try_new_port(&cfg).err(),
            Some(TipcError::SystemError(Error::NoResources)),
            "collide with existing port",
        );

        let path = format!("{}.port.{}{}", SRV_PATH_BASE, "test", MAX_USER_HANDLES);
        let cfg = PortCfg::new(path).unwrap();
        expect_eq!(
            Channel::try_new_port(&cfg).err(),
            Some(TipcError::SystemError(Error::NoResources)),
            "max number of ports reached",
        );
    }

    #[test]
    fn wait_on_port() {
        let mut channels: Vec<Rc<Channel>> = Vec::new();

        for i in first_free_handle_index()..MAX_USER_HANDLES {
            let path = format!("{}.port.{}{}", SRV_PATH_BASE, "test", i);
            let cfg = PortCfg::new(path).unwrap();
            let channel = Channel::try_new_port(&cfg);
            expect!(channel.is_ok(), "create ports");
            channels.try_push(channel.unwrap()).unwrap();
        }

        for chan in &channels {
            expect_eq!(
                chan.handle.wait(Some(0)).err(),
                Some(TipcError::SystemError(Error::TimedOut)),
                "zero timeout",
            );

            expect_eq!(
                chan.handle.wait(Some(100)).err(),
                Some(TipcError::SystemError(Error::TimedOut)),
                "non-zero timeout",
            );
        }
    }

    impl<'s> Serialize<'s> for i32 {
        fn serialize<'a: 's, S: Serializer<'s>>(
            &'a self,
            serializer: &mut S,
        ) -> core::result::Result<S::Ok, S::Error> {
            unsafe { serializer.serialize_as_bytes(self) }
        }
    }

    impl Deserialize for i32 {
        type Error = TipcError;

        const MAX_SERIALIZED_SIZE: usize = 4;

        fn deserialize(
            bytes: &[u8],
            _handles: &mut [Option<Handle>],
        ) -> core::result::Result<Self, Self::Error> {
            Ok(i32::from_ne_bytes(bytes[0..4].try_into().map_err(|_| TipcError::OutOfBounds)?))
        }
    }

    struct Service1;

    impl Service for Service1 {
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
            handle: &Handle,
            _msg: Self::Message,
        ) -> Result<bool> {
            handle.send(&1i32)?;
            Ok(true)
        }
    }

    struct Service2;

    impl Service for Service2 {
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
            handle: &Handle,
            _msg: Self::Message,
        ) -> Result<bool> {
            handle.send(&2i32)?;
            Ok(true)
        }
    }

    service_dispatcher! {
        enum TestServiceDispatcher {
            Service1,
            Service2,
        }
    }

    #[test]
    fn multiple_services() {
        let mut dispatcher = TestServiceDispatcher::<2>::new().unwrap();

        let path1 = format!("{}.port.{}", SRV_PATH_BASE, "testService1");
        let cfg = PortCfg::new(&path1).unwrap();
        dispatcher.add_service(Rc::new(Service1), cfg).expect("Could not add service 1");

        let path2 = format!("{}.port.{}", SRV_PATH_BASE, "testService2");
        let cfg = PortCfg::new(&path2).unwrap();
        dispatcher.add_service(Rc::new(Service2), cfg).expect("Could not add service 2");

        let buffer = [0u8; 4096];
        Manager::<_, _, 2, 4>::new_with_dispatcher(dispatcher, buffer)
            .expect("Could not create service manager");
    }
}

#[cfg(test)]
mod multiservice_with_lifetimes_tests {
    use super::*;
    use core::marker::PhantomData;
    use trusty_std::alloc::FallibleVec;

    const SRV_PATH_BASE: &str = "com.android.ipc-unittest-lifetimes";

    struct Service1<'a> {
        phantom: PhantomData<&'a u32>,
    }

    impl<'a> Service for Service1<'a> {
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
            handle: &Handle,
            _msg: Self::Message,
        ) -> Result<bool> {
            handle.send(&2i32)?;
            Ok(true)
        }
    }

    struct Service2<'a> {
        phantom: PhantomData<&'a u32>,
    }

    impl<'a> Service for Service2<'a> {
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
            handle: &Handle,
            _msg: Self::Message,
        ) -> Result<bool> {
            handle.send(&2i32)?;
            Ok(true)
        }
    }

    service_dispatcher! {
        enum TestServiceLifetimeDispatcher<'a> {
            Service1<'a>,
            Service2<'a>,
        }
    }

    #[test]
    fn manager_creation() {
        let mut dispatcher = TestServiceLifetimeDispatcher::<2>::new().unwrap();

        let path1 = format!("{}.port.{}", SRV_PATH_BASE, "testService1");
        let cfg = PortCfg::new(&path1).unwrap();
        dispatcher
            .add_service(Rc::new(Service1 { phantom: PhantomData }), cfg)
            .expect("Could not add service 1");

        let path2 = format!("{}.port.{}", SRV_PATH_BASE, "testService2");
        let cfg = PortCfg::new(&path2).unwrap();
        dispatcher
            .add_service(Rc::new(Service2 { phantom: PhantomData }), cfg)
            .expect("Could not add service 2");

        let buffer = [0u8; 4096];
        Manager::<_, _, 2, 4>::new_with_dispatcher(dispatcher, buffer)
            .expect("Could not create service manager");
    }
}
