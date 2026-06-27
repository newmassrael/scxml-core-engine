// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: a5d5c62df04659924e14ff2b6c6771228646739eefc82472964b6d7b318ffce2
// generated-at: 1782568712
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
