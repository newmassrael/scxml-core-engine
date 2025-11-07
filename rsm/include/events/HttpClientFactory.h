// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "events/IHttpClient.h"
#include <memory>

namespace SCE {

/**
 * @brief Factory function for creating HTTP clients
 *
 * Creates platform-specific HTTP client implementations:
 * - WASM: EmscriptenFetchClient (uses Fetch API or Node.js http module)
 * - Native: CppHttplibClient (uses cpp-httplib)
 *
 * @return Unique pointer to an IHttpClient implementation
 */
std::unique_ptr<IHttpClient> createHttpClient();

}  // namespace SCE
