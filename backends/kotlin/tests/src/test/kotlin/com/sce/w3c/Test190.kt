// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 2d53d2f6482bd48bbe534a774432c7132f924eed253d3c01ee5b53a731642f97
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test190.scxml:1
package com.sce.w3c

import com.sce.generated.test190.Test190Event
import com.sce.generated.test190.Test190State
import com.sce.generated.test190.Test190StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: [When using the scxml event i/o processor] If the target is the special term '#_scxml_sessionid', where sessionid is the id of an SCXML session that is accessible to the Processor, the Processor MUST add the event to the external queue of that session.
@DisplayName("Test 190 -- W3C SCXML C.1")
class Test190 : W3CTestBase<Test190State, Test190Event>() {
    override fun createStateMachine() = Test190StateMachine(createEngine())
    override val expectedPassState: Test190State = Test190State.Pass
}
