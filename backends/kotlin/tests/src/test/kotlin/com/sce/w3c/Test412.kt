// SCE-GENERATED — DO NOT EDIT
// source-hash: c004aa6d76072d31e9273eed0dc88dc847e351ce7e35f90cc9d56908f01328bd
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
