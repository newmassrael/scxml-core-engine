// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 66bc1c3694f90e60100c842d2a53cd8c05682260c1809ba387d157940d7d6e1d
// generated-at: 1780836426
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
