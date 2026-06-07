// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 66bc1c3694f90e60100c842d2a53cd8c05682260c1809ba387d157940d7d6e1d
// generated-at: 1780836426
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
