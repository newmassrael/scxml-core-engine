// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "GuardUtils.h"

namespace SCE {
namespace GuardUtils {

bool isConditionExpression(const std::string &expression) {
    // Check for operators commonly found in conditional expressions
    return expression.find('>') != std::string::npos || expression.find('<') != std::string::npos ||
           expression.find('=') != std::string::npos || expression.find('!') != std::string::npos ||
           expression.find('+') != std::string::npos || expression.find('-') != std::string::npos ||
           expression.find('*') != std::string::npos || expression.find('/') != std::string::npos;
}

}  // namespace GuardUtils
}  // namespace SCE
