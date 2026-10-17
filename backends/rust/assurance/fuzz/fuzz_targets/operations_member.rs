// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| sce_rust_assurance::targets::feed("operations-member", data));
