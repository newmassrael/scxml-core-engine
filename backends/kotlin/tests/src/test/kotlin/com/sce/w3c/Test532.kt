// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: bf9f012bd8272e352f46f4d8064cf0cf3b743ab6fffdf8c941cc03f3254cb15f
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
