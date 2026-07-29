// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: aa58405544015ba4d1b8207b13e783fe4f4b991c1d05b4cc1602d85ec7348310
// generated-at: 1785339169
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test309.scxml:1
package com.sce.w3c

import com.sce.generated.test309.Test309Event
import com.sce.generated.test309.Test309State
import com.sce.generated.test309.Test309StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.9: If a conditional expression cannot be evaluated as a boolean value ('true' or 'false') or if its evaluation causes an error, the SCXML processor MUST treat the expression as if it evaluated to 'false'.
@DisplayName("Test 309 -- W3C SCXML 5.9")
class Test309 : W3CTestBase<Test309State, Test309Event>() {
    override fun createStateMachine() = Test309StateMachine(createEngine())
    override val expectedPassState: Test309State = Test309State.Pass
}
