// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 00f78dbe00f429352a6571b71d3b75d9ea5e69ddb859956bf6433b48017951ce
// generated-at: 1780031382
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test456.scxml:1
package com.sce.w3c

import com.sce.generated.test456.Test456Event
import com.sce.generated.test456.Test456State
import com.sce.generated.test456.Test456StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: the SCXML Processor must accept any ECMAScript program as defined in Section 14 of [ECMASCRIPT-262] as the content of a script element.
@DisplayName("Test 456 -- W3C SCXML B.2")
class Test456 : W3CTestBase<Test456State, Test456Event>() {
    override fun createStateMachine() = Test456StateMachine(createEngine())
    override val expectedPassState: Test456State = Test456State.Pass
}
