// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-RSM-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "events/IHttpClient.h"

#ifdef __EMSCRIPTEN__
#include "events/EmscriptenFetchClient.h"
#else
#include "events/CppHttplibClient.h"
#endif

namespace RSM {

std::unique_ptr<IHttpClient> createHttpClient() {
#ifdef __EMSCRIPTEN__
    return std::make_unique<EmscriptenFetchClient>();
#else
    return std::make_unique<CppHttplibClient>();
#endif
}

}  // namespace RSM
