// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e798da33d5279236b681cdea18a53a3971a9b769ae5a0bc652a7f8fc89ca7b27
// generated-at: 1779450894
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test452.scxml:1
package com.sce.w3c

import com.sce.generated.test452.Test452Event
import com.sce.generated.test452.Test452State
import com.sce.generated.test452.Test452StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel, the SCXML Processor must accept any ECMAScript left-hand-side expression as a location expression.
@DisplayName("Test 452 -- W3C SCXML B.2")
class Test452 : W3CTestBase<Test452State, Test452Event>() {
    override fun createStateMachine() = Test452StateMachine(createEngine())
    override val expectedPassState: Test452State = Test452State.Pass
}
