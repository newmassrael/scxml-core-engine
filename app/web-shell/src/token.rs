// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The bearer token that stands between a page in some other tab and the works
//! folder.

/// The shortest token accepted from the environment. A generated one is longer.
pub const MIN_LENGTH: usize = 16;

/// 32 random bytes, hex.
pub fn generate() -> Result<String, getrandom::Error> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Whether `offered` is `expected`, without the time depending on where they differ.
pub fn matches(expected: &str, offered: &str) -> bool {
    let (a, b) = (expected.as_bytes(), offered.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

/// The token in an `Authorization: Bearer <token>` header value.
pub fn bearer(header_value: &str) -> Option<&str> {
    let (scheme, token) = header_value.split_once(' ')?;
    (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then_some(token.trim())
}
