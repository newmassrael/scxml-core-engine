// SCE-GENERATED — DO NOT EDIT
// source-hash: a95595288fbbf7d3f81db3dd016df1d833ecb9e57bfe2f3659a4cadecdc82d28
// source: test569.scxml
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test569.scxml:1
package com.sce.w3c

import com.sce.generated.test569.Test569Event
import com.sce.generated.test569.Test569State
import com.sce.generated.test569.Test569StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: For the _ioprocessors system variable in the ECMAScript datamodel the Processor MUST create an array with an object for each Event I/O processor that it supports, where the name of the object is the same as that of the I/O processor. For the SCXML and BasicHTTP Event I/O processors, the Processor MUST create a location property under the object, assigning the access URI as its String value.
@DisplayName("Test 569 -- W3C SCXML B.2")
class Test569 : W3CTestBase<Test569State, Test569Event>() {
    override fun createStateMachine() = Test569StateMachine(createEngine())
    override val expectedPassState: Test569State = Test569State.Pass
}
