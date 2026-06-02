// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: bc7b5b1dd90f65e6c3a4df2e3c4223cf8922d7e6b2d5d124b66683d16074cb6e
// generated-at: 1780362263
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test302.scxml:1
package com.sce.w3c

import com.sce.generated.test302.Test302Event
import com.sce.generated.test302.Test302State
import com.sce.generated.test302.Test302StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.8: The SCXML Processor MUST evaluate any script element that is a child of scxml at document load time. N.B. This test is valid only for datamodels that support scripting.
@DisplayName("Test 302 -- W3C SCXML 5.8")
class Test302 : W3CTestBase<Test302State, Test302Event>() {
    override fun createStateMachine() = Test302StateMachine(createEngine())
    override val expectedPassState: Test302State = Test302State.Pass
}
