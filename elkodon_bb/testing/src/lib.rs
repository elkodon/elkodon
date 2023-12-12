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

#[macro_use]
pub mod assert;
pub mod watchdog;

#[macro_export(local_inner_macros)]
macro_rules! test_requires {
    { $condition:expr } => {
        if !$condition { return; }
    }
}

pub const AT_LEAST_TIMING_VARIANCE: f32 = elkodon_pal_settings::settings::AT_LEAST_TIMING_VARIANCE;
