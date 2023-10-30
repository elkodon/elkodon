use crate::{
    port::port_identifiers::UniqueNotifierId,
    service::{self, event_concept_name},
};
use elkodon_bb_lock_free::mpmc::{container::ContainerState, unique_index_set::UniqueIndex};
use elkodon_bb_log::{fail, warn};
use elkodon_cal::named_concept::NamedConceptBuilder;
use elkodon_cal::{dynamic_storage::DynamicStorage, event::NotifierBuilder};
use std::{cell::UnsafeCell, marker::PhantomData};

use super::{event_id::EventId, port_identifiers::UniqueListenerId};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum NotifierCreateError {
    ExceedsMaxSupportedNotifiers,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum NotifierConnectionUpdateFailure {
    OnlyPartialUpdate,
}

#[derive(Debug, Default)]
struct ListenerConnections<'global_config, Service: service::Details<'global_config>> {
    #[allow(clippy::type_complexity)]
    connections:
        Vec<UnsafeCell<Option<<Service::Event as elkodon_cal::event::Event<EventId>>::Notifier>>>,
}

impl<'global_config, Service: service::Details<'global_config>>
    ListenerConnections<'global_config, Service>
{
    fn new(size: usize) -> Self {
        let mut new_self = Self {
            connections: vec![],
        };

        new_self.connections.reserve(size);
        for _ in 0..size {
            new_self.connections.push(UnsafeCell::new(None))
        }

        new_self
    }

    fn create(&self, index: usize, listener_id: UniqueListenerId) -> Result<(), ()> {
        let event_name = event_concept_name(&listener_id);
        if self.get(index).is_none() {
            let notifier = fail!(from self, when <Service::Event as elkodon_cal::event::Event<EventId>>::NotifierBuilder::new(&event_name).open(),
                                    with (),
                                    "Unable to establish a connection to Listener port {:?}.", listener_id);
            *self.get_mut(index) = Some(notifier);
        }

        Ok(())
    }

    fn get(
        &self,
        index: usize,
    ) -> &Option<<Service::Event as elkodon_cal::event::Event<EventId>>::Notifier> {
        unsafe { &(*self.connections[index].get()) }
    }

    #[allow(clippy::mut_from_ref)]
    fn get_mut(
        &self,
        index: usize,
    ) -> &mut Option<<Service::Event as elkodon_cal::event::Event<EventId>>::Notifier> {
        unsafe { &mut (*self.connections[index].get()) }
    }

    fn len(&self) -> usize {
        self.connections.len()
    }

    fn remove(&self, index: usize) {
        *self.get_mut(index) = None;
    }
}

#[derive(Debug)]
pub struct Notifier<'a, 'global_config: 'a, Service: service::Details<'global_config>> {
    listener_connections: ListenerConnections<'global_config, Service>,
    listener_list_state: UnsafeCell<ContainerState<'a, UniqueListenerId>>,
    default_trigger_id: EventId,
    _dynamic_config_guard: Option<UniqueIndex<'a>>,
    _phantom_a: PhantomData<&'a Service>,
    _phantom_b: PhantomData<&'global_config ()>,
}

impl<'a, 'global_config: 'a, Service: service::Details<'global_config>>
    Notifier<'a, 'global_config, Service>
{
    pub(crate) fn new(
        service: &'a Service,
        default_trigger_id: EventId,
    ) -> Result<Self, NotifierCreateError> {
        let msg = "Unable to create Notifier port";
        let origin = "Notifier::new()";
        let port_id = UniqueNotifierId::new();

        let listener_list = &service.state().dynamic_storage.get().event().listeners;

        let mut new_self = Self {
            listener_connections: ListenerConnections::new(listener_list.capacity()),
            default_trigger_id,
            listener_list_state: unsafe { UnsafeCell::new(listener_list.get_state()) },
            _dynamic_config_guard: None,
            _phantom_a: PhantomData,
            _phantom_b: PhantomData,
        };

        // !MUST! be the last task otherwise a publisher is added to the dynamic config without the
        // creation of all required resources
        let _dynamic_config_guard = match service
            .state()
            .dynamic_storage
            .get()
            .event()
            .add_notifier_id(port_id)
        {
            Some(unique_index) => unique_index,
            None => {
                fail!(from origin, with NotifierCreateError::ExceedsMaxSupportedNotifiers,
                            "{} since it would exceed the maximum supported amount of notifiers of {}.",
                            msg, service.state().static_config.event().max_notifiers);
            }
        };

        new_self._dynamic_config_guard = Some(_dynamic_config_guard);

        if let Err(e) = new_self.populate_listener_channels() {
            warn!(from new_self, "The new Notifier port is unable to connect to every Listener port, caused by {:?}.", e);
        }

        Ok(new_self)
    }

    fn update_connections(&self) -> Result<(), NotifierConnectionUpdateFailure> {
        if unsafe { (*self.listener_list_state.get()).update() } {
            fail!(from self, when self.populate_listener_channels(),
                with NotifierConnectionUpdateFailure::OnlyPartialUpdate,
                "Connections were updated only partially since at least one connection to a Listener port failed.");
        }

        Ok(())
    }

    fn populate_listener_channels(&self) -> Result<(), ()> {
        let mut visited_indices = vec![];
        visited_indices.resize(self.listener_connections.len(), None);

        unsafe {
            (*self.listener_list_state.get()).for_each(|index, listener_id| {
                visited_indices[index as usize] = Some(*listener_id);
            })
        };

        for (i, index) in visited_indices.iter().enumerate() {
            match index {
                Some(listener_id) => match self.listener_connections.create(i, *listener_id) {
                    Ok(()) => (),
                    Err(()) => {
                        fail!(from self, with (),
                            "Unable to establish connection to Listener port {:?}.", *listener_id);
                    }
                },
                None => self.listener_connections.remove(i),
            }
        }

        Ok(())
    }

    pub fn notify(&self) -> Result<usize, NotifierConnectionUpdateFailure> {
        self.notify_with_custom_trigger_id(self.default_trigger_id)
    }

    pub fn notify_with_custom_trigger_id(
        &self,
        value: EventId,
    ) -> Result<usize, NotifierConnectionUpdateFailure> {
        fail!(from self, when self.update_connections(),
            "Unable to notify event since the connections could not be updated.");

        use elkodon_cal::event::Notifier;
        let mut number_of_triggered_listeners = 0;

        for i in 0..self.listener_connections.len() {
            match self.listener_connections.get(i) {
                Some(ref connection) => match connection.notify(value) {
                    Err(e) => {
                        warn!(from self, "Unable to send notification via connection {:?} due to {:?}.", connection, e)
                    }
                    Ok(_) => {
                        number_of_triggered_listeners += 1;
                    }
                },
                None => (),
            }
        }

        Ok(number_of_triggered_listeners)
    }
}
