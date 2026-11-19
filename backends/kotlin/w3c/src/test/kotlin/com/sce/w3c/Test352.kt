// SCE-GENERATED — DO NOT EDIT
// source-hash: 24c0c790790b7fc4233175c81a50eccfcbe14faedc71c39da3d226dd0ede08db
// source: test352.scxml
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test352.scxml:1
package com.sce.w3c

import com.sce.generated.test352.Test352Event
import com.sce.generated.test352.Test352State
import com.sce.generated.test352.Test352StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: 'sourcetype'. The sending SCXML Processor MUST assign this attribute the value "scxml". (Note that other types of senders will assign different values.) The receiving Processor MUST use this value as the value of the 'origintype' field of the event that it generates.
@DisplayName("Test 352 -- W3C SCXML C.1")
class Test352 : W3CTestBase<Test352State, Test352Event>() {
    override fun createStateMachine() = Test352StateMachine(createEngine())
    override val expectedPassState: Test352State = Test352State.Pass
}
