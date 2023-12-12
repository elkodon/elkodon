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

//! Relocatable (inter-process shared memory compatible) [`semantic_string::SemanticString`] implementations for
//! [`GroupName`].
//!
//! # Example
//!
//! ```
//! use iceoryx2_bb_container::semantic_string::SemanticString;
//! use iceoryx2_bb_system_types::group_name::*;
//!
//! let group = GroupName::new(b"some-group").expect("invalid group name");
//!
//! let invalid_group = GroupName::new(b"some*!?group");
//! assert!(invalid_group.is_err());
//! ```

use iceoryx2_bb_container::semantic_string;

const GROUP_NAME_LENGTH: usize = 31;
semantic_string! {
  name: GroupName,
  capacity: GROUP_NAME_LENGTH,
  invalid_content: |string: &[u8]| {
    if string.is_empty() {
        return true;
    }

    matches!(string[0], b'-' | b'0'..=b'9')
  },
  invalid_characters: |string: &[u8]| {
    for value in string {
        match value {
            b'a'..=b'z' | b'0'..=b'9' | b'-' => (),
            _ => return true,
        }
    }

    false
  },
  comparision: |lhs: &[u8], rhs: &[u8]| {
      *lhs == *rhs
  }
}
