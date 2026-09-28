// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "events/EventDescriptor.h"
#include "common/EventDataHelper.h"

namespace SCE {

std::string EventDescriptor::payload() const {
    // W3C SCXML 5.6.2 + B.2 (test 561): the output of <content> is the
    // message's data, and it takes precedence over any other source.
    if (!content.empty()) {
        return content;
    }

    if (data.empty() && params.empty() && typedParams.empty()) {
        return "";
    }

    // SCXML Compliance: "processor MUST reformat this data to match its data model,
    // but MUST NOT otherwise modify it"
    if (!data.empty() && params.empty() && typedParams.empty()) {
        return data;
    }

    // §scxml-5.10: `buildEventDataJson`, not `buildJsonFromParams`: the latter
    // stringifies every value, which is how `<param expr="42"/>` reached a
    // receiver as `"42"`. The typed map is what carries the number.
    if (data.empty()) {
        return EventDataHelper::buildEventDataJson(params, typedParams);
    }

    // For data alongside parameters the same helper composes both halves, so
    // the presence of a `data` attribute cannot change a param's type.
    return EventDataHelper::buildEventDataJson(data, params, typedParams);
}

}  // namespace SCE
