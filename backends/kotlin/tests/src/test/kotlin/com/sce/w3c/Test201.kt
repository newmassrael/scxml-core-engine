// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: e273e083fd84459760e6b7e00629aa0bbc396fdd49f2f0b96778152f02d02625
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test201.scxml:1
package com.sce.w3c

import com.sce.generated.test201.Test201Event
import com.sce.generated.test201.Test201State
import com.sce.generated.test201.Test201StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: Processors that support HTTP POST must use the value http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor for the "type" attribute
@DisplayName("Test 201 -- W3C SCXML 6.2")
class Test201 : W3CHttpTestBase<Test201State, Test201Event>() {
    override fun createStateMachine() = Test201StateMachine(createEngine())
    override val expectedPassState: Test201State = Test201State.Pass
}
