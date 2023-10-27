use elkodon::service::{process_local, zero_copy};
use elkodon::service::{service_name::ServiceName, Service};
use elkodon_bb_container::semantic_string::SemanticString;
use elkodon_bb_log::set_log_level;
use elkodon_bb_posix::barrier::BarrierHandle;
use elkodon_bb_posix::{barrier::BarrierBuilder, clock::Time};

const ITERATIONS: u64 = 10000000;

fn perform_benchmark<T: Service>() {
    let service_name_a2b = ServiceName::new(b"a2b").unwrap();
    let service_name_b2a = ServiceName::new(b"b2a").unwrap();

    let service_a2b = T::new(&service_name_a2b)
        .publish_subscribe()
        .max_publishers(1)
        .max_subscribers(1)
        .history_size(0)
        .subscriber_buffer_size(1)
        .enable_safe_overflow(false)
        .create::<u64>()
        .unwrap();

    let service_b2a = T::new(&service_name_b2a)
        .publish_subscribe()
        .max_publishers(1)
        .max_subscribers(1)
        .history_size(0)
        .subscriber_buffer_size(1)
        .enable_safe_overflow(false)
        .create::<u64>()
        .unwrap();

    let barrier_handle = BarrierHandle::new();
    let barrier = BarrierBuilder::new(3).create(&barrier_handle).unwrap();

    std::thread::scope(|s| {
        let t1 = s.spawn(|| {
            let sender_a2b = service_a2b.publisher().create().unwrap();
            let receiver_b2a = service_b2a.subscriber().create().unwrap();

            barrier.wait();

            for i in 0..ITERATIONS {
                while sender_a2b.send_copy(i).expect("failed to send") == 0 {}

                while receiver_b2a.receive().unwrap().is_none() {}
            }
        });

        let t2 = s.spawn(|| {
            let sender_b2a = service_b2a.publisher().create().unwrap();
            let receiver_a2b = service_a2b.subscriber().create().unwrap();

            barrier.wait();

            for i in 0..ITERATIONS {
                while receiver_a2b.receive().unwrap().is_none() {}

                while sender_b2a.send_copy(i).expect("failed to send") == 0 {}
            }
        });

        std::thread::sleep(std::time::Duration::from_millis(100));
        let start = Time::now().expect("failed to acquire time");
        barrier.wait();

        t1.join().expect("thread failure");
        t2.join().expect("thread failure");
        let stop = start.elapsed().expect("failed to measure time");
        println!(
            "{} ::: Time: {}, Latency: {} ns",
            std::any::type_name::<T>(),
            stop.as_secs_f64(),
            stop.as_nanos() / (ITERATIONS as u128 * 2)
        );
    });
}

fn main() {
    set_log_level(elkodon_bb_log::LogLevel::Error);
    perform_benchmark::<zero_copy::Service>();
    perform_benchmark::<process_local::Service>();
}
