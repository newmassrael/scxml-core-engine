// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <string>

namespace SCE {

// §scxml-B-2: Helper functions for data content processing

// Normalize whitespace in text content (test 558)
std::string normalizeWhitespace(const std::string &text);

// Detect if content is XML (test 557)
bool isXMLContent(const std::string &content);

}  // namespace SCE
