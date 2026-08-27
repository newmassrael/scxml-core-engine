// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: a432b690c7990abdc6b5ce0526e592fee5b7d55e84a37b350376bb446a9dc3cf
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test331.scxml:1
package com.sce.w3c

import com.sce.generated.test331.Test331Event
import com.sce.generated.test331.Test331State
import com.sce.generated.test331.Test331StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The SCXML Processor MUST set the type property of _event to: "platform" (for events raised by the platform itself, such as error events), "internal" (for events raised by raise and send with target '_internal') or "external" (for all other events).
@DisplayName("Test 331 -- W3C SCXML 5.10")
class Test331 : W3CTestBase<Test331State, Test331Event>() {
    override fun createStateMachine() = Test331StateMachine(createEngine())
    override val expectedPassState: Test331State = Test331State.Pass
}
