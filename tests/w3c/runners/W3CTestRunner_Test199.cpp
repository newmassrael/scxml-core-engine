/**
 * @brief W3C SCXML 6.2: Unsupported send type raises error.execution
 *
 * Validates that using an unsupported event I/O processor type in <send> element
 * correctly raises error.execution event on internal queue.
 *
 * Test Scenario:
 * - <send type="unsupported_type"> triggers error.execution
 * - error.execution has higher priority than timeout event
 * - Transition on error.execution → pass state
 *
 * ARCHITECTURE.md Compliance:
 * - Pure Static AOT: Fully compile-time state machine structure
 * - Zero Duplication: Uses SendHelper for send type validation
 * - All-or-Nothing: Complete static code generation
 *
 * W3C SCXML References:
 * - 6.2: Send element type attribute validation
 * - 5.10.1: Internal vs external event queue priority
 * - 5.10: Error event generation (error.execution)
 */
#include "aot_tests/Test199.h"
