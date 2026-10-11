// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
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
