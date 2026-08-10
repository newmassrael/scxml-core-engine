// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $5000 cumulative
//   Enterprise: Contact for pricing
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "common/SCXMLConstants.h"
#include "common/UrlEncodingHelper.h"
#include <string>
#include <vector>

namespace SCE {

/**
 * @brief One entry of the `_ioprocessors` system variable
 *
 * `name` is the key the entry is filed under; `location` is the value of the
 * entry's 'location' field, an address external entities can use to reach this
 * session through that processor.
 */
struct IOProcessorDescriptor {
    std::string name;
    std::string location;
};

/**
 * @brief Builds the `_ioprocessors` entry set for a session
 *
 * ARCHITECTURE.md Zero Duplication: the entry set is decided here once and
 * handed to whichever script engine backs the session, so the Interpreter and
 * the AOT engines cannot disagree about which processors a session advertises
 * or what address each advertises.
 *
 * Every processor is filed twice: under the specification's entry name and
 * under the short alias. The alias is what SCXML documents in the field index
 * with (`_ioprocessors['scxml'].location`), and the W3C conformance suite's
 * conf: vocabulary expands to the alias form as well, so an implementation that
 * published only the specification name would be unusable from the very
 * documents the entry exists to serve. Both keys carry the same location, so
 * the choice of spelling never changes where an event goes.
 *
 * Which processors appear is a property of the deployment, not of the document:
 * the SCXML Event I/O Processor is always present because the engine always
 * carries it, while the Basic HTTP processor appears only once an inbound
 * BasicHTTP endpoint has actually been declared for the session. A session with
 * no HTTP endpoint deployed advertises no HTTP entry rather than an address
 * nothing is listening on.
 */
class IOProcessorHelper {
public:
    /// Alias the SCXML Event I/O Processor is indexed under by SCXML documents.
    static constexpr const char *SCXML_ALIAS = "scxml";

    /// Alias the Basic HTTP Event I/O Processor is indexed under by SCXML documents.
    static constexpr const char *BASIC_HTTP_ALIAS = "basichttp";

    /**
     * @brief Address that reaches this session over the SCXML Event I/O Processor
     *
     * §scxml-C-1 leaves the transport platform-specific, so the address is an
     * SCE-scheme URI naming the session. The session id is percent-encoded
     * because it reaches this point from `<invoke>` and from embedder-supplied
     * ids, neither of which is constrained to URI-safe characters.
     */
    static std::string scxmlLocation(const std::string &sessionId) {
        return "sce://scxml/" + UrlEncodingHelper::urlEncode(sessionId);
    }

    /**
     * @brief Session id an SCXML Event I/O Processor location names, if any
     *
     * The inverse of scxmlLocation, kept beside it so the two spellings of
     * one address cannot drift apart. §scxml-C-1 requires the location a
     * session publishes to be usable as a <send> target ("the 'origin' field
     * ... MUST match the 'location' field"), which only holds if something
     * can read a session back out of it.
     *
     * @param uri Candidate location
     * @return Decoded session id, or empty when `uri` is not an SCXML
     *         processor location or names no session
     */
    static std::string sessionIdFromScxmlLocation(const std::string &uri) {
        static constexpr const char *kPrefix = "sce://scxml/";
        const size_t prefixLen = std::char_traits<char>::length(kPrefix);
        if (uri.size() <= prefixLen || uri.compare(0, prefixLen, kPrefix) != 0) {
            return "";
        }
        return UrlEncodingHelper::urlDecode(uri.substr(prefixLen));
    }

    /**
     * @brief Entry set for a session
     *
     * §scxml-C-1-1: the SCXML Event I/O Processor entry and its 'location'
     * field are always maintained.
     *
     * §scxml-C-2-3: the Basic HTTP Event I/O Processor entry and its 'location'
     * field are maintained whenever that processor is supported. Support is
     * per-deployment — it exists exactly when the embedder has declared the
     * inbound access URI — so an empty `basicHttpAccessUri` produces no entry.
     *
     * @param sessionId Session the entries describe
     * @param basicHttpAccessUri Inbound BasicHTTP access URI, empty if none is deployed
     */
    static std::vector<IOProcessorDescriptor> build(const std::string &sessionId,
                                                    const std::string &basicHttpAccessUri = "") {
        std::vector<IOProcessorDescriptor> descriptors;

        const std::string scxmlUri = scxmlLocation(sessionId);
        descriptors.push_back({Constants::SCXML_EVENT_PROCESSOR_TYPE, scxmlUri});
        descriptors.push_back({SCXML_ALIAS, scxmlUri});

        if (!basicHttpAccessUri.empty()) {
            descriptors.push_back({Constants::BASIC_HTTP_EVENT_PROCESSOR_URI, basicHttpAccessUri});
            descriptors.push_back({BASIC_HTTP_ALIAS, basicHttpAccessUri});
        }

        return descriptors;
    }
};

}  // namespace SCE
