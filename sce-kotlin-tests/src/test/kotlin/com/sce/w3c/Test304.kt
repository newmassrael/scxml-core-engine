// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 00f78dbe00f429352a6571b71d3b75d9ea5e69ddb859956bf6433b48017951ce
// generated-at: 1780031382
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test304.scxml:1
package com.sce.w3c

import com.sce.generated.test304.Test304Event
import com.sce.generated.test304.Test304State
import com.sce.generated.test304.Test304StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.8: In a conformant SCXML document, the name of any script variable MAY be used as a location expression. N.B. This test is valid only for datamodels that support scripting.
@DisplayName("Test 304 -- W3C SCXML 5.8")
class Test304 : W3CTestBase<Test304State, Test304Event>() {
    override fun createStateMachine() = Test304StateMachine(createEngine())
    override val expectedPassState: Test304State = Test304State.Pass
}
