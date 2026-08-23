// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: c96808b03e7b119d29792dbf258f9125c91be8c72d4823c8f9b56e0e05a3240b
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
