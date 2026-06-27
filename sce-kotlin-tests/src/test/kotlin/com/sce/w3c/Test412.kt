// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 4a741a2915b4fc1d6292d4cc68ddf4af4e269ea63531bfee3c7b94ccd4e9b0bc
// generated-at: 1782562648
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test412.scxml:1
package com.sce.w3c

import com.sce.generated.test412.Test412Event
import com.sce.generated.test412.Test412State
import com.sce.generated.test412.Test412StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: If the state is a default entry state and has an initial child, the SCXML Processor MUST then [after doing the active state add and the onentry handlers] execute the executable content in the initial child's transition.
@DisplayName("Test 412 -- W3C SCXML 3.13")
class Test412 : W3CTestBase<Test412State, Test412Event>() {
    override fun createStateMachine() = Test412StateMachine()
    override val expectedPassState: Test412State = Test412State.Pass
    override val timeoutMs: Long = 5000L
}
