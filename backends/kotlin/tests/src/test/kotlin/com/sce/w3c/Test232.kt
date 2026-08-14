// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b82119528bc210fbc6e453d658ae079f31e3529ce331b1d6045090bb79eaa2ff
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test232.scxml:1
package com.sce.w3c

import com.sce.generated.test232.Test232Event
import com.sce.generated.test232.Test232State
import com.sce.generated.test232.Test232StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: he invoked external service MAY return multiple events while it is processing
@DisplayName("Test 232 -- W3C SCXML 6.4")
class Test232 : W3CTestBase<Test232State, Test232Event>() {
    override fun createStateMachine() = Test232StateMachine()
    override val expectedPassState: Test232State = Test232State.Pass
}
