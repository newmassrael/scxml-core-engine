// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: beb72c3a9cb76e61aa4916ff585cb6a1d22e66c189bf8cc96c5023dec391d982
// generated-at: 1780379958
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test451.scxml:1
package com.sce.w3c

import com.sce.generated.test451.Test451Event
import com.sce.generated.test451.Test451State
import com.sce.generated.test451.Test451StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel, the SCXML Processor must add an ECMAScript
@DisplayName("Test 451 -- W3C SCXML B.2")
class Test451 : W3CTestBase<Test451State, Test451Event>() {
    override fun createStateMachine() = Test451StateMachine()
    override val expectedPassState: Test451State = Test451State.Pass
}
