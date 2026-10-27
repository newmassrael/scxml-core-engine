// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The SCE_LOG_* macros, with SCE::Logger as their sink (docs/adr/0060).
//
// Every SCE source file that logs includes this header, never core/LogMacros.h
// directly. It defines one `Logger::SinkInit` in the file, as <iostream> defines a
// `std::ios_base::Init`: the object of a file that logs is linked exactly when its code
// is used, and the SinkInit runs before any function of that file does, so SCE's own
// messages reach the configured backend from the first one on, as they did when the
// macros called the Logger directly. A sink the program installed first is kept.
//
// Including it binds the program to sce_base, which defines the Logger. Header-only
// code includes core/LogMacros.h, which binds it to nothing.

#pragma once

#include "common/Logger.h"
#include "core/LogMacros.h"

namespace SCE::detail {

[[maybe_unused]] static const Logger::SinkInit loggerSinkInit;

}  // namespace SCE::detail
