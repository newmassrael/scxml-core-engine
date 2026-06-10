// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: aa3f7478a78abf9bf22f51a549ae822f834be956298adbc33316f195f470808d
// generated-at: 1781099318
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test223.scxml:1
package com.sce.w3c

import com.sce.generated.test223.Test223Event
import com.sce.generated.test223.Test223State
import com.sce.generated.test223.Test223StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If the 'idlocation' attribute is present, the SCXML Processor MUST generate an id automatically when the invoke element is evaluated and store it in the location specified by 'idlocation'.
@DisplayName("Test 223 -- W3C SCXML 6.4")
class Test223 : W3CTestBase<Test223State, Test223Event>() {
    override fun createStateMachine() = Test223StateMachine(createEngine())
    override val expectedPassState: Test223State = Test223State.Pass
}
