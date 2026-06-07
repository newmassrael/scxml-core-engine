// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 66bc1c3694f90e60100c842d2a53cd8c05682260c1809ba387d157940d7d6e1d
// generated-at: 1780836426
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
