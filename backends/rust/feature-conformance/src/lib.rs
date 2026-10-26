// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The machines `build.rs` generates from `product-line/session.scxml`
//! (docs/adr/0057): [`lifted`], one machine for every build, and the machine of
//! each feature set generated alone. The tests hold the first to the second.

#![allow(clippy::all)]

include!(concat!(env!("OUT_DIR"), "/machines.rs"));

// `variant` is the machine this crate's own feature set selects, generated
// alone: what a consumer that built only this configuration would have
// generated. One of the three is compiled, so each stands under the set it names.
#[cfg(feature = "session-unicast-accept")]
pub use variant_accept as variant;
#[cfg(not(feature = "session-unicast"))]
pub use variant_none as variant;
#[cfg(all(feature = "session-unicast", not(feature = "session-unicast-accept")))]
pub use variant_unicast as variant;
