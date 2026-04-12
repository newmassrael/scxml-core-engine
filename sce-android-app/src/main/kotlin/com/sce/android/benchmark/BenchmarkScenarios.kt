// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Android — Port of all JMH benchmark scenarios to Android-compatible lambdas
//
// Each BenchmarkScenario maps 1:1 to a JMH @Benchmark method.
// The harness calls setup -> warmup(body) -> measure(body) -> teardown.

package com.sce.android.benchmark

import com.sce.runtime.ScxmlScriptEngine

data class BenchmarkScenario(
    val name: String,
    val category: String,
    val setup: (ScxmlScriptEngine, String) -> Unit = { _, _ -> },
    val body: (ScxmlScriptEngine, String) -> Unit,
    val teardown: (ScxmlScriptEngine, String) -> Unit = { _, _ -> }
)

object BenchmarkScenarios {

    val all: List<BenchmarkScenario> by lazy {
        session() + expression() + script() + variable() + dataParse() + scenario()
    }

    val categories: List<String> by lazy {
        all.map { it.category }.distinct()
    }

    // -----------------------------------------------------------------------
    // Session lifecycle (maps to SessionBenchmark.kt)
    //
    // Session benchmarks are special: they create/destroy sessions within
    // the body itself. The harness-level session is used only for counter state.
    // -----------------------------------------------------------------------
    private fun session(): List<BenchmarkScenario> {
        var counter = 0

        return listOf(
            BenchmarkScenario(
                name = "createSetupDestroy",
                category = "Session",
                setup = { engine, sid ->
                    counter = 0
                },
                body = { engine, _ ->
                    val sid = "bench_session_${++counter}"
                    engine.createSession(sid)
                    engine.setupSystemVariables(sid, "benchmark")
                    engine.destroySession(sid)
                }
            ),
            BenchmarkScenario(
                name = "createDestroyOnly",
                category = "Session",
                setup = { _, _ ->
                    counter = 0
                },
                body = { engine, _ ->
                    val sid = "bench_bare_${++counter}"
                    engine.createSession(sid)
                    engine.destroySession(sid)
                }
            )
        )
    }

    // -----------------------------------------------------------------------
    // Expression evaluation (maps to ExpressionBenchmark.kt)
    // Guard conditions are on the CRITICAL PATH of every state transition.
    // -----------------------------------------------------------------------
    private fun expression(): List<BenchmarkScenario> {
        val exprSetup: (ScxmlScriptEngine, String) -> Unit = { engine, sid ->
            engine.setVariable(sid, "Var1", 42)
            engine.setVariable(sid, "Var2", 7)
            engine.setVariable(sid, "Var3", "hello")
            engine.executeScript(sid, "var counter = 0;")
        }

        return listOf(
            BenchmarkScenario(
                name = "conditionSimple",
                category = "Expression",
                setup = exprSetup,
                body = { engine, sid -> engine.evaluateCondition(sid, "Var1 == 42") }
            ),
            BenchmarkScenario(
                name = "conditionCompound",
                category = "Expression",
                setup = exprSetup,
                body = { engine, sid -> engine.evaluateCondition(sid, "Var1 > 0 && Var2 < 100") }
            ),
            BenchmarkScenario(
                name = "conditionNegation",
                category = "Expression",
                setup = exprSetup,
                body = { engine, sid -> engine.evaluateCondition(sid, "!(Var1 == 0)") }
            ),
            BenchmarkScenario(
                name = "exprArithmetic",
                category = "Expression",
                setup = exprSetup,
                body = { engine, sid -> engine.evaluateExpr(sid, "1 + 2 * 3") }
            ),
            BenchmarkScenario(
                name = "exprMathBuiltins",
                category = "Expression",
                setup = exprSetup,
                body = { engine, sid -> engine.evaluateExpr(sid, "Math.sqrt(144) + Math.pow(2, 10)") }
            ),
            BenchmarkScenario(
                name = "exprStringConcat",
                category = "Expression",
                setup = exprSetup,
                body = { engine, sid -> engine.evaluateExpr(sid, "'hello' + ' ' + 'world'") }
            ),
            BenchmarkScenario(
                name = "exprVarArithmetic",
                category = "Expression",
                setup = exprSetup,
                body = { engine, sid -> engine.evaluateExpr(sid, "Var1 + Var2 * 2") }
            ),
            BenchmarkScenario(
                name = "exprTernary",
                category = "Expression",
                setup = exprSetup,
                body = { engine, sid -> engine.evaluateExpr(sid, "Var1 > 0 ? Var1 : -Var1") }
            )
        )
    }

    // -----------------------------------------------------------------------
    // Script execution (maps to ScriptBenchmark.kt)
    // -----------------------------------------------------------------------
    private fun script(): List<BenchmarkScenario> {
        val tinyScript = "result = result + 1;"

        val smallScript = buildString {
            appendLine("var sum = 0;")
            for (i in 1..5) appendLine("sum = sum + $i;")
            appendLine("result = sum;")
        }

        val mediumScript = buildString {
            appendLine("var arr = [];")
            appendLine("for (var i = 0; i < 20; i++) { arr.push(i * 2); }")
            appendLine("var total = 0;")
            appendLine("for (var j = 0; j < arr.length; j++) { total = total + arr[j]; }")
            appendLine("result = total;")
        }

        val largeScript = buildString {
            appendLine("var data = {};")
            for (i in 0 until 50) appendLine("data['key$i'] = $i * $i;")
            appendLine("var sum = 0;")
            appendLine("for (var k in data) { sum = sum + data[k]; }")
            appendLine("result = sum;")
        }

        val scriptSetup: (ScxmlScriptEngine, String) -> Unit = { engine, sid ->
            engine.executeScript(sid, "var result = 0;")
        }

        return listOf(
            BenchmarkScenario(
                name = "scriptTiny",
                category = "Script",
                setup = scriptSetup,
                body = { engine, sid -> engine.executeScript(sid, tinyScript) }
            ),
            BenchmarkScenario(
                name = "scriptSmall",
                category = "Script",
                setup = scriptSetup,
                body = { engine, sid -> engine.executeScript(sid, smallScript) }
            ),
            BenchmarkScenario(
                name = "scriptMedium",
                category = "Script",
                setup = scriptSetup,
                body = { engine, sid -> engine.executeScript(sid, mediumScript) }
            ),
            BenchmarkScenario(
                name = "scriptLarge",
                category = "Script",
                setup = scriptSetup,
                body = { engine, sid -> engine.executeScript(sid, largeScript) }
            )
        )
    }

    // -----------------------------------------------------------------------
    // Variable operations (maps to VariableBenchmark.kt)
    // -----------------------------------------------------------------------
    private fun variable(): List<BenchmarkScenario> {
        val varSetup: (ScxmlScriptEngine, String) -> Unit = { engine, sid ->
            engine.setVariable(sid, "target", 0)
            engine.setVariable(sid, "counter", 100)
        }

        return listOf(
            BenchmarkScenario(
                name = "setVariable",
                category = "Variable",
                setup = varSetup,
                body = { engine, sid -> engine.setVariable(sid, "target", 42) }
            ),
            BenchmarkScenario(
                name = "getVariable",
                category = "Variable",
                setup = varSetup,
                body = { engine, sid -> engine.getVariable(sid, "counter") }
            ),
            BenchmarkScenario(
                name = "assignExpression",
                category = "Variable",
                setup = varSetup,
                body = { engine, sid -> engine.assign(sid, "target", "counter + 1") }
            ),
            BenchmarkScenario(
                name = "hasVariable",
                category = "Variable",
                setup = varSetup,
                body = { engine, sid -> engine.hasVariable(sid, "counter") }
            ),
            BenchmarkScenario(
                name = "initializeDataModel",
                category = "Variable",
                setup = varSetup,
                body = { engine, sid ->
                    engine.setVariable(sid, "v1", 0)
                    engine.setVariable(sid, "v2", "text")
                    engine.setVariable(sid, "v3", true)
                    engine.setVariable(sid, "v4", 3.14)
                    engine.setVariable(sid, "v5", null)
                }
            )
        )
    }

    // -----------------------------------------------------------------------
    // Data parsing (maps to DataParseBenchmark.kt)
    // -----------------------------------------------------------------------
    private fun dataParse(): List<BenchmarkScenario> {
        val jsonSimple = """{"name": "test", "value": 42}"""
        val jsonComplex = """{"users": [{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}], "count": 2}"""
        val xmlSimple = """<data><item key="a">1</item><item key="b">2</item></data>"""
        val plainText = "  hello   world   this  is  a  test  "

        return listOf(
            BenchmarkScenario(
                name = "parseJsonSimple",
                category = "DataParse",
                body = { engine, sid -> engine.parseDataValue(sid, jsonSimple) }
            ),
            BenchmarkScenario(
                name = "parseJsonComplex",
                category = "DataParse",
                body = { engine, sid -> engine.parseDataValue(sid, jsonComplex) }
            ),
            BenchmarkScenario(
                name = "parseXml",
                category = "DataParse",
                body = { engine, sid -> engine.parseDataValue(sid, xmlSimple) }
            ),
            BenchmarkScenario(
                name = "parsePlainText",
                category = "DataParse",
                body = { engine, sid -> engine.parseDataValue(sid, plainText) }
            )
        )
    }

    // -----------------------------------------------------------------------
    // Realistic SCXML scenarios (maps to ScxmlScenarioBenchmark.kt)
    // -----------------------------------------------------------------------
    private fun scenario(): List<BenchmarkScenario> {
        val scenarioSetup: (ScxmlScriptEngine, String) -> Unit = { engine, sid ->
            val activeStates = setOf("s0", "s1", "running")
            engine.setStateQueryCallback(sid) { stateId -> stateId in activeStates }
            engine.setVariable(sid, "Var1", 0)
            engine.setVariable(sid, "Var2", 10)
            engine.setVariable(sid, "Var3", "hello")
            engine.executeScript(sid, "var items = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];")
            engine.executeScript(sid, "var itemResult = 0;")
        }

        val scenarioTeardown: (ScxmlScriptEngine, String) -> Unit = { engine, sid ->
            engine.setStateQueryCallback(sid, null)
        }

        return listOf(
            BenchmarkScenario(
                name = "eventProcessingCycle",
                category = "Scenario",
                setup = scenarioSetup,
                teardown = scenarioTeardown,
                body = { engine, sid ->
                    engine.setCurrentEvent(
                        sid,
                        name = "user.click",
                        data = "",
                        type = "external",
                        sendId = "",
                        origin = "#_scxml_bench",
                        originType = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor",
                        invokeId = ""
                    )
                    val guard = engine.evaluateCondition(sid, "_event.name == 'user.click'")
                    if (guard) {
                        engine.assign(sid, "Var1", "Var1 + 1")
                    }
                    engine.clearCurrentEvent(sid)
                }
            ),
            BenchmarkScenario(
                name = "guardWithVariables",
                category = "Scenario",
                setup = scenarioSetup,
                teardown = scenarioTeardown,
                body = { engine, sid ->
                    engine.evaluateCondition(sid, "Var1 >= 0 && Var2 < 100 && Var3 == 'hello'")
                }
            ),
            BenchmarkScenario(
                name = "inPredicate",
                category = "Scenario",
                setup = scenarioSetup,
                teardown = scenarioTeardown,
                body = { engine, sid ->
                    engine.evaluateCondition(sid, "In('running')")
                }
            ),
            BenchmarkScenario(
                name = "foreachIteration",
                category = "Scenario",
                setup = scenarioSetup,
                teardown = scenarioTeardown,
                body = { engine, sid ->
                    engine.executeForeach(
                        sid,
                        array = "items",
                        item = "x",
                        index = "idx",
                        body = { engine.assign(sid, "itemResult", "itemResult + x") }
                    )
                }
            ),
            BenchmarkScenario(
                name = "fullMicrostep",
                category = "Scenario",
                setup = scenarioSetup,
                teardown = scenarioTeardown,
                body = { engine, sid ->
                    engine.setCurrentEvent(
                        sid,
                        name = "timer.elapsed",
                        data = """{"count": 5}""",
                        type = "external"
                    )
                    engine.evaluateCondition(sid, "_event.name == 'error'")
                    val matched = engine.evaluateCondition(
                        sid, "_event.name == 'timer.elapsed' && Var2 > 0"
                    )
                    if (matched) {
                        engine.executeScript(sid, "var _saved = Var1;")
                        engine.assign(sid, "Var1", "Var1 + 1")
                        engine.assign(sid, "Var2", "Var2 - 1")
                        engine.executeScript(sid, "var _entered = true;")
                    }
                    engine.clearCurrentEvent(sid)
                }
            ),
            BenchmarkScenario(
                name = "dataModelInit",
                category = "Scenario",
                setup = scenarioSetup,
                teardown = scenarioTeardown,
                body = { engine, sid ->
                    engine.setVariable(sid, "d1", 0)
                    engine.setVariable(sid, "d2", null)
                    engine.assign(sid, "d1", "42 * 2")
                    engine.executeScript(sid, "var d3 = [1, 2, 3]; var d4 = {'key': 'value'};")
                    engine.evaluateExpr(sid, "d1 + 1")
                }
            )
        )
    }
}
