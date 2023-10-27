use elkodon::service::{service_name::ServiceName, zero_copy, Service};
use elkodon_bb_container::semantic_string::SemanticString;
use elkodon_bb_posix::signal::SignalHandler;
use example_publish_subscribe_transmission_data::TransmissionData;

fn main() {
    let service_name = ServiceName::new(b"My/Funk/ServiceName").unwrap();

    let service = zero_copy::Service::new(&service_name)
        .publish_subscribe()
        .open_or_create::<TransmissionData>()
        .expect("failed to create/open service");

    let publisher = service
        .publisher()
        .create()
        .expect("failed to create publisher");

    let mut counter: u64 = 0;

    while !SignalHandler::was_ctrl_c_pressed() {
        // send by copy
        publisher
            .send_copy(TransmissionData {
                x: counter as i32,
                y: counter as i32 * 10,
                funky: 789.123 * counter as f64,
            })
            .expect("failed to send sample");

        // zero copy send
        let mut sample = publisher.loan().expect("Failed to acquire sample");
        unsafe {
            sample.as_mut_ptr().write(TransmissionData {
                x: counter as i32,
                y: counter as i32 * 3,
                funky: counter as f64 * 812.12,
            });
        }
        publisher.send(sample).expect("Failed to send sample");

        counter += 1;
        println!("Send sample {} ...", counter);

        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    println!("exit ...");
}
