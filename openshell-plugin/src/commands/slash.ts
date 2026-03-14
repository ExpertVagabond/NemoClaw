// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

/**
 * Handler for the /openshell slash command (chat interface).
 *
 * Supports subcommands:
 *   /openshell status   - show sandbox/blueprint/inference state
 *   /openshell eject    - rollback to host installation
 *   /openshell          - show help
 */

import type { PluginCommandContext, PluginCommandResult, OpenClawPluginApi } from "../index.js";
import { loadState } from "../blueprint/state.js";

export function handleSlashCommand(
  ctx: PluginCommandContext,
  _api: OpenClawPluginApi,
): PluginCommandResult {
  const subcommand = ctx.args?.trim().split(/\s+/)[0] ?? "";

  switch (subcommand) {
    case "status":
      return slashStatus();
    case "eject":
      return slashEject();
    default:
      return slashHelp();
  }
}

function slashHelp(): PluginCommandResult {
  return {
    text: [
      "**OpenShell Plugin**",
      "",
      "Usage: `/openshell <subcommand>`",
      "",
      "Subcommands:",
      "  `status` - Show sandbox, blueprint, and inference state",
      "  `eject`  - Show rollback instructions",
      "",
      "For full management use the CLI:",
      "  `openclaw openshell status`",
      "  `openclaw openshell migrate`",
      "  `openclaw openshell launch`",
      "  `openclaw openshell connect`",
      "  `openclaw openshell eject --confirm`",
    ].join("\n"),
  };
}

function slashStatus(): PluginCommandResult {
  const state = loadState();

  if (!state.lastAction) {
    return {
      text: "**OpenShell Plugin**: No operations performed yet. Run `openclaw openshell launch` or `openclaw openshell migrate` to get started.",
    };
  }

  const lines = [
    "**OpenShell Plugin Status**",
    "",
    `Last action: ${state.lastAction}`,
    `Blueprint: ${state.blueprintVersion ?? "unknown"}`,
    `Run ID: ${state.lastRunId ?? "none"}`,
    `Sandbox: ${state.sandboxName ?? "none"}`,
    `Updated: ${state.updatedAt}`,
  ];

  if (state.migrationSnapshot) {
    lines.push("", `Rollback snapshot: ${state.migrationSnapshot}`);
  }

  return { text: lines.join("\n") };
}

function slashEject(): PluginCommandResult {
  const state = loadState();

  if (!state.lastAction) {
    return { text: "No OpenShell deployment found. Nothing to eject from." };
  }

  if (!state.migrationSnapshot && !state.hostBackupPath) {
    return {
      text: "No migration snapshot found. Manual rollback required.",
    };
  }

  return {
    text: [
      "**Eject from OpenShell**",
      "",
      "To rollback to your host OpenClaw installation, run:",
      "",
      "```",
      "openclaw openshell eject --confirm",
      "```",
      "",
      `Snapshot: ${state.migrationSnapshot ?? state.hostBackupPath ?? "none"}`,
    ].join("\n"),
  };
}
