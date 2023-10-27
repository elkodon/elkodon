use elkodon::service::{zero_copy, Details};

fn main() {
    let services = zero_copy::Service::list().expect("failed to acquire list of current services");

    for service in services {
        println!("\n{:#?}", &service);
    }
}
