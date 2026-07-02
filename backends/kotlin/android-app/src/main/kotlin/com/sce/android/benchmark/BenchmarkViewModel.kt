// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Android — ViewModel driving benchmark execution on background threads

package com.sce.android.benchmark

import android.util.Log
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.sce.android.EngineType
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

data class BenchmarkState(
    val isRunning: Boolean = false,
    val progress: BenchmarkProgress? = null,
    val results: List<BenchmarkResult> = emptyList(),
    val selectedEngines: Set<EngineType> = EngineType.entries.toSet(),
    val selectedCategory: String? = null,
    val errorMessage: String? = null
)

class BenchmarkViewModel : ViewModel() {

    companion object {
        private const val TAG = "SCE_BENCH"
    }

    private val _state = MutableStateFlow(BenchmarkState())
    val state: StateFlow<BenchmarkState> = _state.asStateFlow()

    private val harness = BenchmarkHarness()
    private var runJob: Job? = null

    fun toggleEngine(type: EngineType) {
        val current = _state.value.selectedEngines
        val updated = if (type in current && current.size > 1) {
            current - type
        } else {
            current + type
        }
        _state.value = _state.value.copy(selectedEngines = updated)
    }

    fun selectCategory(category: String?) {
        _state.value = _state.value.copy(selectedCategory = category)
    }

    fun runBenchmarks() {
        if (_state.value.isRunning) return

        runJob = viewModelScope.launch {
            _state.value = _state.value.copy(
                isRunning = true,
                results = emptyList(),
                errorMessage = null
            )

            val scenarios = filteredScenarios()
            val engines = _state.value.selectedEngines.toList()
            val totalCount = scenarios.size * engines.size
            var completedCount = 0
            val results = mutableListOf<BenchmarkResult>()

            for (scenario in scenarios) {
                for (engine in engines) {
                    _state.value = _state.value.copy(
                        progress = BenchmarkProgress(
                            currentScenario = scenario.name,
                            currentEngine = engine,
                            completedCount = completedCount,
                            totalCount = totalCount
                        )
                    )

                    try {
                        val result = withContext(Dispatchers.Default) {
                            harness.run(scenario, engine)
                        }
                        results.add(result)
                        Log.i(TAG, "RESULT | %-12s | %-25s | %10.2f us/op | %10.0f ops/s".format(
                            engine.label, "${scenario.category}/${scenario.name}",
                            result.meanUs, result.opsPerSec
                        ))
                    } catch (e: Exception) {
                        results.add(
                            BenchmarkResult(
                                scenarioName = scenario.name,
                                category = scenario.category,
                                engineType = engine,
                                meanUs = Double.NaN,
                                medianUs = Double.NaN,
                                stddevUs = Double.NaN,
                                p99Us = Double.NaN,
                                minUs = Double.NaN,
                                maxUs = Double.NaN,
                                opsPerSec = 0.0
                            )
                        )
                        Log.e(TAG, "ERROR  | %-12s | %-25s | %s".format(
                            engine.label, "${scenario.category}/${scenario.name}", e.message
                        ))
                    }

                    completedCount++
                    _state.value = _state.value.copy(results = results.toList())
                }
            }

            Log.i(TAG, "========== BENCHMARK COMPLETE ==========")
            _state.value = _state.value.copy(isRunning = false, progress = null)
        }
    }

    fun stopBenchmarks() {
        runJob?.cancel()
        _state.value = _state.value.copy(isRunning = false, progress = null)
    }

    private fun filteredScenarios(): List<BenchmarkScenario> {
        val category = _state.value.selectedCategory
        return if (category == null) {
            BenchmarkScenarios.all
        } else {
            BenchmarkScenarios.all.filter { it.category == category }
        }
    }
}
