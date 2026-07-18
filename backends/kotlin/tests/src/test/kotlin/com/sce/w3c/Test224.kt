// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2337021aa5cf9b8209b5932f23ab0e04a6899271e435f3620bc1da41d7c4d7b7
// generated-at: 1784381545
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test224.scxml:1
package com.sce.w3c

import com.sce.generated.test224.Test224Event
import com.sce.generated.test224.Test224State
import com.sce.generated.test224.Test224StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: When the platform generates an identifier for 'idlocation', the identifier MUST have the form stateid.platformid, where stateid is the id of the state containing this element and platformid is automatically generated.
@DisplayName("Test 224 -- W3C SCXML 6.4")
class Test224 : W3CTestBase<Test224State, Test224Event>() {
    override fun createStateMachine() = Test224StateMachine(createEngine())
    override val expectedPassState: Test224State = Test224State.Pass
    override val timeoutMs: Long = 5000L
}
