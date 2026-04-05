// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 newmassrael
//
// SCE Android — Compose UI for benchmark execution and results display

package com.sce.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.sce.android.EngineType
import com.sce.android.benchmark.BenchmarkResult
import com.sce.android.benchmark.BenchmarkScenarios
import com.sce.android.benchmark.BenchmarkState
import com.sce.android.benchmark.BenchmarkViewModel

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun BenchmarkScreen(viewModel: BenchmarkViewModel) {
    val state by viewModel.state.collectAsState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("SCE Engine Benchmark") }
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = 16.dp)
        ) {
            // Engine selection chips
            Text(
                text = "Engines",
                style = MaterialTheme.typography.labelLarge,
                modifier = Modifier.padding(top = 8.dp)
            )
            FlowRow(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.padding(vertical = 4.dp)
            ) {
                EngineType.entries.forEach { engine ->
                    FilterChip(
                        selected = engine in state.selectedEngines,
                        onClick = { viewModel.toggleEngine(engine) },
                        label = { Text(engine.label) },
                        enabled = !state.isRunning
                    )
                }
            }

            // Category dropdown
            CategorySelector(
                selectedCategory = state.selectedCategory,
                enabled = !state.isRunning,
                onSelect = { viewModel.selectCategory(it) }
            )

            Spacer(modifier = Modifier.height(8.dp))

            // Run / Stop buttons
            Row(
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                if (state.isRunning) {
                    OutlinedButton(
                        onClick = { viewModel.stopBenchmarks() },
                        modifier = Modifier.weight(1f)
                    ) {
                        Text("Stop")
                    }
                } else {
                    Button(
                        onClick = { viewModel.runBenchmarks() },
                        modifier = Modifier.weight(1f)
                    ) {
                        Text("Run Benchmarks")
                    }
                }
            }

            // Progress
            state.progress?.let { progress ->
                Spacer(modifier = Modifier.height(8.dp))
                LinearProgressIndicator(
                    progress = { progress.completedCount.toFloat() / progress.totalCount },
                    modifier = Modifier.fillMaxWidth()
                )
                Text(
                    text = "${progress.currentEngine.label}: ${progress.currentScenario} " +
                           "(${progress.completedCount}/${progress.totalCount})",
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.padding(top = 4.dp)
                )
            }

            Spacer(modifier = Modifier.height(12.dp))

            // Results table
            if (state.results.isNotEmpty()) {
                ResultsTable(
                    results = state.results,
                    engines = state.selectedEngines.toList().sorted(),
                    modifier = Modifier.weight(1f)
                )
            }
        }
    }
}

@Composable
private fun CategorySelector(
    selectedCategory: String?,
    enabled: Boolean,
    onSelect: (String?) -> Unit
) {
    var expanded by remember { mutableStateOf(false) }
    val categories = listOf("All") + BenchmarkScenarios.categories

    Box {
        TextButton(
            onClick = { expanded = true },
            enabled = enabled
        ) {
            Text("Category: ${selectedCategory ?: "All"}")
        }
        DropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false }
        ) {
            categories.forEach { cat ->
                DropdownMenuItem(
                    text = { Text(cat) },
                    onClick = {
                        onSelect(if (cat == "All") null else cat)
                        expanded = false
                    }
                )
            }
        }
    }
}

@Composable
private fun ResultsTable(
    results: List<BenchmarkResult>,
    engines: List<EngineType>,
    modifier: Modifier = Modifier
) {
    // Group results by scenario name
    val grouped = results.groupBy { it.scenarioName }

    val scrollState = rememberScrollState()

    Column(modifier = modifier) {
        // Header
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .horizontalScroll(scrollState)
                .background(MaterialTheme.colorScheme.surfaceVariant)
                .padding(vertical = 8.dp, horizontal = 4.dp)
        ) {
            Text(
                text = "Benchmark",
                fontWeight = FontWeight.Bold,
                fontSize = 12.sp,
                modifier = Modifier.width(160.dp)
            )
            engines.forEach { engine ->
                Text(
                    text = "${engine.label}\n(us/op)",
                    fontWeight = FontWeight.Bold,
                    fontSize = 12.sp,
                    textAlign = TextAlign.End,
                    modifier = Modifier.width(90.dp)
                )
            }
        }

        HorizontalDivider()

        // Rows
        LazyColumn {
            val scenarioNames = grouped.keys.toList()
            items(scenarioNames) { scenarioName ->
                val scenarioResults = grouped[scenarioName] ?: return@items
                val resultsByEngine = scenarioResults.associateBy { it.engineType }

                // Find fastest engine for color coding
                val validMeans = engines.mapNotNull { eng ->
                    resultsByEngine[eng]?.meanUs?.takeIf { !it.isNaN() }?.let { eng to it }
                }
                val fastestEngine = validMeans.minByOrNull { it.second }?.first

                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .horizontalScroll(scrollState)
                        .padding(vertical = 6.dp, horizontal = 4.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    // Scenario name with category prefix
                    val category = scenarioResults.first().category
                    Text(
                        text = "$category/\n$scenarioName",
                        fontSize = 11.sp,
                        fontFamily = FontFamily.Monospace,
                        lineHeight = 14.sp,
                        modifier = Modifier.width(160.dp)
                    )

                    // Engine values
                    engines.forEach { engine ->
                        val result = resultsByEngine[engine]
                        val color = when {
                            result == null || result.meanUs.isNaN() -> MaterialTheme.colorScheme.error
                            engine == fastestEngine -> MaterialTheme.colorScheme.primary
                            else -> MaterialTheme.colorScheme.onSurface
                        }
                        val weight = if (engine == fastestEngine) FontWeight.Bold else FontWeight.Normal

                        Text(
                            text = if (result != null && !result.meanUs.isNaN()) {
                                formatUs(result.meanUs)
                            } else {
                                "ERR"
                            },
                            fontSize = 12.sp,
                            fontFamily = FontFamily.Monospace,
                            fontWeight = weight,
                            color = color,
                            textAlign = TextAlign.End,
                            modifier = Modifier.width(90.dp)
                        )
                    }
                }

                HorizontalDivider(thickness = 0.5.dp)
            }
        }
    }
}

private fun formatUs(value: Double): String {
    return when {
        value >= 1000 -> String.format("%.0f", value)
        value >= 100 -> String.format("%.1f", value)
        value >= 10 -> String.format("%.2f", value)
        else -> String.format("%.3f", value)
    }
}
