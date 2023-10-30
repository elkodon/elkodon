#[generic_tests::define]
mod service_event {
    use elkodon::global_config::{Config, Entries};
    use elkodon::port::event_id::EventId;
    use elkodon::service::{
        builder::event::{EventCreateError, EventOpenError},
        service_name::ServiceName,
        Service,
    };
    use elkodon_bb_container::semantic_string::*;
    use elkodon_bb_posix::unique_system_id::UniqueSystemId;
    use elkodon_bb_testing::assert_that;

    fn generate_name() -> ServiceName {
        let mut service = ServiceName::new(b"service_tests_").unwrap();
        service
            .push_bytes(
                UniqueSystemId::new()
                    .unwrap()
                    .value()
                    .to_string()
                    .as_bytes(),
            )
            .unwrap();
        service
    }

    #[test]
    fn creating_non_existing_service_works<Sut: Service>() {
        let service_name = generate_name();
        let sut = Sut::new(&service_name).event().create();

        assert_that!(sut, is_ok);
        let sut = sut.unwrap();
        assert_that!(*sut.name(), eq service_name);
    }

    #[test]
    fn creating_same_service_twice_fails<Sut: Service>() {
        let service_name = generate_name();
        let sut = Sut::new(&service_name).event().create();
        assert_that!(sut, is_ok);

        let sut2 = Sut::new(&service_name).event().create();
        assert_that!(sut2, is_err);
        assert_that!(
            sut2.err().unwrap(), eq
            EventCreateError::AlreadyExists
        );
    }

    #[test]
    fn recreate_after_drop_works<Sut: Service>() {
        let service_name = generate_name();
        let sut = Sut::new(&service_name).event().create();
        assert_that!(sut, is_ok);

        drop(sut);

        let sut2 = Sut::new(&service_name).event().create();
        assert_that!(sut2, is_ok);
    }

    #[test]
    fn open_fails_when_service_does_not_exist<Sut: Service>() {
        let service_name = generate_name();
        let sut = Sut::new(&service_name).event().open();
        assert_that!(sut, is_err);
        assert_that!(sut.err().unwrap(), eq EventOpenError::DoesNotExist);
    }

    #[test]
    fn open_succeeds_when_service_does_exist<Sut: Service>() {
        let service_name = generate_name();
        let sut = Sut::new(&service_name).event().create();
        assert_that!(sut, is_ok);

        let sut2 = Sut::new(&service_name).event().open();
        assert_that!(sut2, is_ok);
    }

    #[test]
    fn open_fails_when_service_does_not_fulfill_opener_requirements<Sut: Service>() {
        let service_name = generate_name();
        let sut = Sut::new(&service_name)
            .event()
            .max_notifiers(2)
            .max_listeners(2)
            .create();
        assert_that!(sut, is_ok);

        // notifier
        let sut2 = Sut::new(&service_name).event().max_notifiers(3).open();

        assert_that!(sut2, is_err);
        assert_that!(
            sut2.err().unwrap(), eq
            EventOpenError::DoesNotSupportRequestedAmountOfNotifiers
        );

        let sut2 = Sut::new(&service_name).event().max_notifiers(1).open();
        assert_that!(sut2, is_ok);

        // listener
        let sut2 = Sut::new(&service_name).event().max_listeners(3).open();

        assert_that!(sut2, is_err);
        assert_that!(
            sut2.err().unwrap(), eq
            EventOpenError::DoesNotSupportRequestedAmountOfListeners
        );

        let sut2 = Sut::new(&service_name).event().max_listeners(1).open();
        assert_that!(sut2, is_ok);
    }

    #[test]
    fn open_uses_predefined_settings_when_nothing_is_specified<Sut: Service>() {
        let service_name = generate_name();
        let sut = Sut::new(&service_name)
            .event()
            .max_notifiers(4)
            .max_listeners(5)
            .create()
            .unwrap();
        assert_that!(sut.max_supported_notifiers(), eq 4);
        assert_that!(sut.max_supported_listeners(), eq 5);

        let sut2 = Sut::new(&service_name).event().open().unwrap();
        assert_that!(sut2.max_supported_notifiers(), eq 4);
        assert_that!(sut2.max_supported_listeners(), eq 5);
    }

    #[test]
    fn settings_can_be_modified_via_custom_config<Sut: Service>() {
        let service_name = generate_name();
        let mut entries = Entries::default();
        entries.defaults.event.max_notifiers = 9;
        entries.defaults.event.max_listeners = 10;

        let custom_config = Config::from_entries(&entries);
        let sut = Sut::new(&service_name)
            .event_with_custom_config(&custom_config)
            .create()
            .unwrap();
        assert_that!(sut.max_supported_notifiers(), eq 9);
        assert_that!(sut.max_supported_listeners(), eq 10);

        let sut2 = Sut::new(&service_name)
            .event_with_custom_config(&custom_config)
            .open()
            .unwrap();
        assert_that!(sut2.max_supported_notifiers(), eq 9);
        assert_that!(sut2.max_supported_listeners(), eq 10);
    }

    #[test]
    fn simple_communication_works_listener_created_first<Sut: Service>() {
        let service_name = generate_name();
        let event_id = EventId::new(432);

        let sut = Sut::new(&service_name).event().create().unwrap();

        let sut2 = Sut::new(&service_name).event().open().unwrap();

        let mut listener = sut.listener().create().unwrap();
        let notifier = sut2
            .notifier()
            .default_trigger_id(event_id)
            .create()
            .unwrap();

        assert_that!(notifier.notify(), is_ok);

        let mut received_events = 0;
        for event in listener.try_wait().unwrap().iter() {
            assert_that!(*event, eq event_id);
            received_events += 1;
        }
        assert_that!(received_events, eq 1);
    }

    #[test]
    fn simple_communication_works_notifier_created_first<Sut: Service>() {
        let service_name = generate_name();
        let event_id = EventId::new(43212);

        let sut = Sut::new(&service_name).event().create().unwrap();

        let sut2 = Sut::new(&service_name).event().open().unwrap();

        let notifier = sut2
            .notifier()
            .default_trigger_id(event_id)
            .create()
            .unwrap();
        let mut listener = sut.listener().create().unwrap();

        assert_that!(notifier.notify(), is_ok);

        let mut received_events = 0;
        for event in listener.try_wait().unwrap().iter() {
            assert_that!(*event, eq event_id);
            received_events += 1;
        }
        assert_that!(received_events, eq 1);
    }

    #[test]
    fn communication_with_max_notifiers_and_listeners_single_notification<Sut: Service>() {
        const MAX_LISTENERS: usize = 4;
        const MAX_NOTIFIERS: usize = 6;
        const NUMBER_OF_ITERATIONS: u64 = 128;
        let service_name = generate_name();

        let sut = Sut::new(&service_name)
            .event()
            .max_notifiers(MAX_NOTIFIERS)
            .max_listeners(MAX_LISTENERS)
            .create()
            .unwrap();

        let mut listeners = vec![];
        let mut notifiers = vec![];

        for _ in 0..MAX_LISTENERS {
            listeners.push(sut.listener().create().unwrap());
        }

        for i in 0..MAX_NOTIFIERS {
            notifiers.push(
                sut.notifier()
                    .default_trigger_id(EventId::new((4 * i + 3) as u64))
                    .create()
                    .unwrap(),
            );
        }

        for _ in 0..NUMBER_OF_ITERATIONS {
            for (i, notifier) in notifiers.iter().enumerate() {
                assert_that!(notifier.notify(), is_ok);

                for listener in &mut listeners {
                    let mut received_events = 0;
                    for event in listener.try_wait().unwrap().iter() {
                        assert_that!(*event, eq EventId::new((4*i + 3) as u64));
                        received_events += 1;
                    }
                    assert_that!(received_events, eq 1);
                }
            }
        }
    }

    #[test]
    fn communication_with_max_notifiers_and_listeners_multi_notification<Sut: Service>() {
        const MAX_LISTENERS: usize = 5;
        const MAX_NOTIFIERS: usize = 7;
        const NUMBER_OF_ITERATIONS: u64 = 128;
        let service_name = generate_name();

        let sut = Sut::new(&service_name)
            .event()
            .max_notifiers(MAX_NOTIFIERS)
            .max_listeners(MAX_LISTENERS)
            .create()
            .unwrap();

        let mut listeners = vec![];
        let mut notifiers = vec![];

        for _ in 0..MAX_LISTENERS {
            listeners.push(sut.listener().create().unwrap());
        }

        for i in 0..MAX_NOTIFIERS {
            notifiers.push(
                sut.notifier()
                    .default_trigger_id(EventId::new((i) as u64))
                    .create()
                    .unwrap(),
            );
        }

        for _ in 0..NUMBER_OF_ITERATIONS {
            for notifier in &notifiers {
                assert_that!(notifier.notify(), is_ok);
            }

            for listener in &mut listeners {
                let mut received_events = 0;

                let mut received_event_ids = [false; MAX_NOTIFIERS];
                for event in listener.try_wait().unwrap().iter() {
                    assert_that!(received_event_ids[event.as_u64() as usize], eq false);
                    received_event_ids[event.as_u64() as usize] = true;
                    received_events += 1;
                }
                assert_that!(received_events, eq MAX_NOTIFIERS);
            }
        }
    }

    #[instantiate_tests(<elkodon::service::zero_copy::Service>)]
    mod zero_copy {}

    #[instantiate_tests(<elkodon::service::process_local::Service>)]
    mod process_local {}
}
