pub mod event;
pub mod publish_subscribe;

use std::{
    fmt::Display,
    sync::atomic::{AtomicU64, Ordering},
};

use elkodon_bb_log::{fail, fatal_panic};
use elkodon_bb_memory::bump_allocator::BumpAllocator;

const MARKED_FOR_DESTRUCTION: u64 = u64::MAX - 1;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub(crate) enum DecrementReferenceCounterResult {
    HasOwners,
    NoMoreOwners,
}

#[derive(Debug)]
pub enum MessagingPattern {
    PublishSubscribe(publish_subscribe::DynamicConfig),
    Event(event::DynamicConfig),
}

#[derive(Debug)]
pub struct DynamicConfig {
    messaging_pattern: MessagingPattern,
    reference_counter: AtomicU64,
}

impl Display for DynamicConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "service::DynamicConfig {{ messaging_pattern: {:?} }}",
            self.messaging_pattern
        )
    }
}

impl DynamicConfig {
    pub(crate) fn new_uninit(messaging_pattern: MessagingPattern) -> Self {
        Self {
            messaging_pattern,
            reference_counter: AtomicU64::new(1),
        }
    }

    pub(crate) unsafe fn init(&self, allocator: &BumpAllocator) {
        match &self.messaging_pattern {
            MessagingPattern::PublishSubscribe(ref v) => v.init(allocator),
            MessagingPattern::Event(ref v) => v.init(allocator),
        }
    }

    pub(crate) fn increment_reference_counter(&self) -> Result<(), ()> {
        let mut current_value = self.reference_counter.load(Ordering::Relaxed);
        loop {
            if current_value == MARKED_FOR_DESTRUCTION {
                fail!(from self, with (),
                    "Unable to increment reference counter for dynamic config since it is marked for destruction.");
            }

            match self.reference_counter.compare_exchange(
                current_value,
                current_value + 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(v) => current_value = v,
            }
        }

        Ok(())
    }

    pub(crate) fn decrement_reference_counter(&self) -> DecrementReferenceCounterResult {
        let mut result;
        let mut current_value = self.reference_counter.load(Ordering::Relaxed);

        loop {
            result = DecrementReferenceCounterResult::HasOwners;
            match self.reference_counter.compare_exchange(
                current_value,
                if current_value == 1 {
                    result = DecrementReferenceCounterResult::NoMoreOwners;
                    MARKED_FOR_DESTRUCTION
                } else {
                    current_value - 1
                },
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(v) => current_value = v,
            }
        }

        result
    }

    pub(crate) fn publish_subscribe(&self) -> &publish_subscribe::DynamicConfig {
        match &self.messaging_pattern {
            MessagingPattern::PublishSubscribe(ref v) => v,
            m => {
                fatal_panic!(from self, "This should never happen! Try to access publish_subscribe::DynamicConfig when the messaging pattern is actually {:?}.", m);
            }
        }
    }

    pub(crate) fn event(&self) -> &event::DynamicConfig {
        match &self.messaging_pattern {
            MessagingPattern::Event(ref v) => v,
            m => {
                fatal_panic!(from self, "This should never happen! Try to access event::DynamicConfig when the messaging pattern is actually {:?}.", m);
            }
        }
    }
}
