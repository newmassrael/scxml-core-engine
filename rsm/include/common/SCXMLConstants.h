#pragma once

namespace RSM::Constants {

/**
 * @brief W3C SCXML C.1: SCXML Event I/O Processor type URL
 *
 * This constant represents the canonical URL for the SCXML Event I/O Processor
 * as defined in W3C SCXML 1.0 specification Appendix C.1.
 *
 * Used for _event.origintype when events are sent via SCXML Event I/O Processor,
 * including:
 * - Parent-child communication (send target="#_parent")
 * - Invoke event routing
 * - Done.invoke events
 *
 * W3C SCXML allows "scxml" as shorthand, but this is the canonical form.
 *
 * ARCHITECTURE.md: Single Source of Truth for SCXML processor type identification.
 * Shared between Interpreter (ParentEventTarget) and AOT (SendHelper) engines.
 *
 * @see W3C SCXML 1.0 Appendix C.1 "SCXML Event I/O Processor"
 * @see Test 253: SCXML Event I/O Processor bidirectional communication
 */
constexpr const char *SCXML_EVENT_PROCESSOR_TYPE = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor";

}  // namespace RSM::Constants
