// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh: Distributed SCXML transport codegen pipeline.
//
// Reads deploy.yaml topology configuration and generates transport-native
// routing code alongside the existing SM codegen output. See SCE_MESH.md.
//
// Pipeline stages:
//   Deploy   → stage 1 (deploy.yaml parsing in deploy.rs)
//   Topology → stage 2 (SCXML <send> target collection + binding resolution)
//   Codegen  → stage 3 (transport template selection + rendering)

pub mod error;
pub mod deploy;
pub mod pattern;
pub mod topology;
pub mod codegen;
