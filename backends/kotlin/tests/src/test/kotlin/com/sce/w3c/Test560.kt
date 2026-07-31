// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 1785490018
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test560.scxml:1
package com.sce.w3c

import com.sce.generated.test560.Test560Event
import com.sce.generated.test560.Test560State
import com.sce.generated.test560.Test560StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript data model, if the content provided to populate _event.data can be interpeted as key-value pairs, then for each unique key, the SCXML Processor MUST create a property of _event.data whose name is the name of the key-value pair and whose value is the value of the key-value pair.
@DisplayName("Test 560 -- W3C SCXML B.2")
class Test560 : W3CTestBase<Test560State, Test560Event>() {
    override fun createStateMachine() = Test560StateMachine(createEngine())
    override val expectedPassState: Test560State = Test560State.Pass
}
