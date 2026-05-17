// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c1736039ea6628ae1068e428522a9d89bbe2ccef2705503db256c49ec169955e
// generated-at: 1778992486
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test402.scxml:1
package com.sce.w3c

import com.sce.generated.test402.Test402Event
import com.sce.generated.test402.Test402State
import com.sce.generated.test402.Test402StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.12: The processor MUST process them [error events] like any other event.
@DisplayName("Test 402 -- W3C SCXML 3.12")
class Test402 : W3CTestBase<Test402State, Test402Event>() {
    override fun createStateMachine() = Test402StateMachine(createEngine())
    override val expectedPassState: Test402State = Test402State.Pass
}
