// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: a184cafe7a67a901b89b33f0188251221f7a1b0a549681a02c5ff67b5487c305
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test532.scxml:1
package com.sce.w3c

import com.sce.generated.test532.Test532Event
import com.sce.generated.test532.Test532State
import com.sce.generated.test532.Test532StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If _scxmleventname is not present, the Processor MUST the name of the HTTP method that was used to deliver the event as name of the SCXML event that it raises
@DisplayName("Test 532 -- W3C SCXML C.2")
class Test532 : W3CHttpTestBase<Test532State, Test532Event>() {
    override fun createStateMachine() = Test532StateMachine(createEngine())
    override val expectedPassState: Test532State = Test532State.Pass
}
