// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 1785490018
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test558.scxml:1
package com.sce.w3c

import com.sce.generated.test558.Test558Event
import com.sce.generated.test558.Test558State
import com.sce.generated.test558.Test558StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel, if either the 'src' attribute or in-line content is provided in the data element, and the content (whether fetched or provided in-line) is not an XML document or JSON (or the
@DisplayName("Test 558 -- W3C SCXML B.2")
class Test558 : W3CTestBase<Test558State, Test558Event>() {
    override fun createStateMachine() = Test558StateMachine(createEngine())
    override val expectedPassState: Test558State = Test558State.Pass
}
