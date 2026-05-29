// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 00f78dbe00f429352a6571b71d3b75d9ea5e69ddb859956bf6433b48017951ce
// generated-at: 1780031382
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test457.scxml:1
package com.sce.w3c

import com.sce.generated.test457.Test457Event
import com.sce.generated.test457.Test457State
import com.sce.generated.test457.Test457StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript data model, the legal iterable collections are arrays, namely objects that satisfy instanceof(Array) in ECMAScript.  The legal values for the 'item' attribute on foreach are legal ECMAScript variable names.
@DisplayName("Test 457 -- W3C SCXML B.2")
class Test457 : W3CTestBase<Test457State, Test457Event>() {
    override fun createStateMachine() = Test457StateMachine(createEngine())
    override val expectedPassState: Test457State = Test457State.Pass
}
