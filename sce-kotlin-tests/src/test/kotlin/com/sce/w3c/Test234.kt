// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c1736039ea6628ae1068e428522a9d89bbe2ccef2705503db256c49ec169955e
// generated-at: 1778992486
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test234.scxml:1
package com.sce.w3c

import com.sce.generated.test234.Test234Event
import com.sce.generated.test234.Test234State
import com.sce.generated.test234.Test234StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: t MUST NOT execute the finalize handler in any other instance of invoke besides the one in the instance of invoke that created the service that generated the event.
@DisplayName("Test 234 -- W3C SCXML 6.4")
class Test234 : W3CTestBase<Test234State, Test234Event>() {
    override fun createStateMachine() = Test234StateMachine(createEngine())
    override val expectedPassState: Test234State = Test234State.Pass
}
