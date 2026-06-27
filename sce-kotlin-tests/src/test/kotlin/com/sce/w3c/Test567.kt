// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 4a741a2915b4fc1d6292d4cc68ddf4af4e269ea63531bfee3c7b94ccd4e9b0bc
// generated-at: 1782562648
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test567.scxml:1
package com.sce.w3c

import com.sce.generated.test567.Test567Event
import com.sce.generated.test567.Test567State
import com.sce.generated.test567.Test567StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: The processor MUST use any message content other than '_scxmleventname' to populate _event.data.
@DisplayName("Test 567 -- W3C SCXML C.2")
class Test567 : W3CHttpTestBase<Test567State, Test567Event>() {
    override fun createStateMachine() = Test567StateMachine(createEngine())
    override val expectedPassState: Test567State = Test567State.Pass
}
