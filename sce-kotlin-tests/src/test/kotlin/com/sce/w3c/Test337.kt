// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 7b1a0066fa6a7fefceddfcf4d1e81b9d1fe50e95dd2b02645dfe86a65f3b96fe
// generated-at: 1780606838
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test337.scxml:1
package com.sce.w3c

import com.sce.generated.test337.Test337Event
import com.sce.generated.test337.Test337State
import com.sce.generated.test337.Test337StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: For internal and platform events, the Processor MUST leave the origintype field blank.
@DisplayName("Test 337 -- W3C SCXML 5.10")
class Test337 : W3CTestBase<Test337State, Test337Event>() {
    override fun createStateMachine() = Test337StateMachine()
    override val expectedPassState: Test337State = Test337State.Pass
}
