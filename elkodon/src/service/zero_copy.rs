// Copyright (c) 2023 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache Software License 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0, or the MIT license
// which is available at https://opensource.org/licenses/MIT.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! # Example
//!
//! ```
//! use elkodon::prelude::*;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let service_name = ServiceName::new("My/Funk/ServiceName")?;
//!
//! // use `zero_copy` as communication variant
//! let service = zero_copy::Service::new(&service_name)
//!     .publish_subscribe()
//!     .open_or_create::<u64>()?;
//!
//! let publisher = service.publisher().create()?;
//! let subscriber = service.subscriber().create()?;
//!
//! # Ok(())
//! # }
//! ```

use crate::port::event_id::EventId;
use crate::service::dynamic_config::DynamicConfig;
use elkodon_cal::shm_allocator::pool_allocator::PoolAllocator;
use elkodon_cal::*;

use super::ServiceState;

/// Defines a zero copy inter-process communication setup based on posix mechanisms.
#[derive(Debug)]
pub struct Service<'config> {
    state: ServiceState<
        'config,
        static_storage::file::Storage,
        dynamic_storage::posix_shared_memory::Storage<DynamicConfig>,
    >,
}

impl<'config> crate::service::Service for Service<'config> {
    type Type<'b> = Service<'b>;
}

impl<'config> crate::service::Details<'config> for Service<'config> {
    type StaticStorage = static_storage::file::Storage;
    type ConfigSerializer = serialize::toml::Toml;
    type DynamicStorage = dynamic_storage::posix_shared_memory::Storage<DynamicConfig>;
    type ServiceNameHasher = hash::sha1::Sha1;
    type SharedMemory = shared_memory::posix::Memory<PoolAllocator>;
    type Connection = zero_copy_connection::posix_shared_memory::Connection;
    type Event = event::unix_datagram_socket::Event<EventId>;

    fn from_state(state: ServiceState<'config, Self::StaticStorage, Self::DynamicStorage>) -> Self {
        Self { state }
    }

    fn state(&self) -> &ServiceState<'config, Self::StaticStorage, Self::DynamicStorage> {
        &self.state
    }

    fn state_mut(
        &mut self,
    ) -> &mut ServiceState<'config, Self::StaticStorage, Self::DynamicStorage> {
        &mut self.state
    }
}
