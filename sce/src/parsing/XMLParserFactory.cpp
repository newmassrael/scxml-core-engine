// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "core/LogMacros.h"
#include "parsing/IXMLParser.h"
#include "parsing/PugiXMLParser.h"

namespace SCE {

std::shared_ptr<IXMLParser> IXMLParser::create() {
    SCE_LOG_DEBUG("Creating PugiXMLParser (unified for all platforms)");
    return std::make_shared<PugiXMLParser>();
}

}  // namespace SCE
