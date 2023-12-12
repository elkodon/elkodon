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

use core::time::Duration;
use elkodon::prelude::*;

const CYCLE_TIME: Duration = Duration::from_secs(1);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_name = ServiceName::new("MyEventName")?;

    let event = zero_copy::Service::new(&event_name)
        .event()
        .open_or_create()?;

    let notifier = event.notifier().create()?;

    let mut counter: u64 = 0;
    while let ElkEvent::Tick = Elk::wait(CYCLE_TIME) {
        counter += 1;
        notifier.notify_with_custom_event_id(EventId::new(counter))?;

        println!("Trigger event with id {} ...", counter);
    }

    println!("exit ... ");

    Ok(())
}
