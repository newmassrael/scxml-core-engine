// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 newmassrael
//
// SCE Android — Benchmark sample app entry point

package com.sce.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import com.sce.android.benchmark.BenchmarkViewModel
import com.sce.android.ui.BenchmarkScreen
import com.sce.android.ui.theme.SceTheme

class MainActivity : ComponentActivity() {

    private val viewModel: BenchmarkViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            SceTheme {
                BenchmarkScreen(viewModel)
            }
        }
    }
}
