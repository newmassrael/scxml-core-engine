// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. AGPL-3.0 + Linking Exception: Free for unmodified use (see LICENSE-EXCEPTION.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Pricing: contact newmassrael@gmail.com
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "common/SCXMLConstants.h"
#include "common/UrlEncodingHelper.h"
#include "core/EventOrigin.h"
#include <optional>
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
 * @brief An event's origin and sender as one text pair spells them back
 *
 * What `IOProcessorHelper::readStoredOrigin` answers for an `origin` and
 * `origintype` stored as text (saved state formats 1 and 2, a queue
 * snapshot), and what a format-3 saved state holds as written.
 */
struct StoredOrigin {
    std::optional<Core::EventOrigin> origin;
    std::string sender;
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
     * @brief The origin the SCXML Event I/O Processor writes for an event a
     *        session sends (docs/adr/0090, section 2)
     *
     * The origin of an event raised in the receiving session must match the
     * location the sending session publishes (`scxmlLocation`), so the pair is the
     * processor's canonical type and `scxmlLocation` of the sender — the one
     * function that also writes the session's `_ioprocessors` entry. The
     * sender's id itself travels beside the pair, as the event's `sender`,
     * for the lookups that find a session by it.
     *
     * @param senderSessionId The sending session's id
     */
    static Core::EventOrigin scxmlOrigin(const std::string &senderSessionId) {
        return Core::EventOrigin(Constants::SCXML_EVENT_PROCESSOR_TYPE, scxmlLocation(senderSessionId));
    }

    /**
     * @brief `scxmlOrigin` of a sender that may be absent
     *
     * A machine that holds no session id has no address to publish, so an
     * event it sends carries no origin rather than the location of an empty
     * id.
     *
     * @param senderSessionId The sending session's id, empty for none
     * @return The SCXML processor's pair, or std::nullopt for no sender
     */
    static std::optional<Core::EventOrigin> scxmlOriginOf(const std::string &senderSessionId) {
        if (senderSessionId.empty()) {
            return std::nullopt;
        }
        return scxmlOrigin(senderSessionId);
    }

    /**
     * @brief The `origin` text that stores an event's origin and sender
     *
     * docs/adr/0090, section 7: the sending session's id for an event a
     * session sent, the address otherwise, and empty for none — what a saved
     * state read from a format-1 text is written back with, as format 2 wrote
     * its `origin`. The `origintype` beside it is the pair's processor.
     */
    static std::string storedOriginText(const std::optional<Core::EventOrigin> &origin, const std::string &sender) {
        return sender.empty() ? Core::originAddressOf(origin) : sender;
    }

    /**
     * @brief The origin and sender a stored `origin` and `origintype` are read as
     *
     * docs/adr/0090, section 7. An empty `origin` is none whatever type was
     * stored beside it: an empty address answers nothing, and format 2 wrote
     * the SCXML type beside it for events no processor stamped. An empty
     * `origintype` beside an address is the SCXML processor's, which is what
     * format 2 meant by it. A value without a scheme read with the SCXML type
     * is a session's id, read as that session's pair with the id as its
     * sender; an SCXML location (which one backend stored in place of the id)
     * is read the same way by the id it names. Anything else is the pair as
     * stored, with no sender.
     */
    static StoredOrigin readStoredOrigin(const std::string &origin, const std::string &originType) {
        if (origin.empty()) {
            return {};
        }
        const std::string located = sessionIdFromScxmlLocation(origin);
        if (!located.empty()) {
            return {scxmlOrigin(located), located};
        }
        const std::string processor =
            originType.empty() ? std::string(Constants::SCXML_EVENT_PROCESSOR_TYPE) : originType;
        const bool bareId = origin.find("://") == std::string::npos;
        if (bareId && processor == Constants::SCXML_EVENT_PROCESSOR_TYPE) {
            return {scxmlOrigin(origin), origin};
        }
        return {Core::EventOrigin(processor, origin), std::string()};
    }

    /**
     * @brief The sender a stored waiting send's `origin` names
     *
     * docs/adr/0090, section 7: a waiting send is always a session's, so a
     * bare id is that id and an SCXML location names it.
     */
    static std::string readStoredSender(const std::string &origin) {
        const std::string located = sessionIdFromScxmlLocation(origin);
        return located.empty() ? origin : located;
    }

    /**
     * @brief The canonical type URI a `<send type>` spelling names
     *
     * `_ioprocessors` files each processor under its type URI and under its
     * short alias, and a document may write either as a `<send>` type. The
     * processor an event is delivered by is named by its URI alone, so the
     * alias is read as the URI where the type is resolved and `origintype`
     * never carries the author's spelling (docs/adr/0090, section 4). Any
     * other value is returned as written.
     *
     * @param sendType The `type` a `<send>` wrote or its `typeexpr` produced
     */
    static std::string canonicalProcessorType(const std::string &sendType) {
        if (sendType == SCXML_ALIAS) {
            return Constants::SCXML_EVENT_PROCESSOR_TYPE;
        }
        if (sendType == BASIC_HTTP_ALIAS) {
            return Constants::BASIC_HTTP_EVENT_PROCESSOR_URI;
        }
        return sendType;
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
