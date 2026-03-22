// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import type { PluginLogger } from "../index.js";

export type BlueprintAction = "plan" | "apply" | "status" | "rollback";

export interface BlueprintRunOptions {
  blueprintPath: string;
  action: BlueprintAction;
  profile: string;
  planPath?: string;
  runId?: string;
  jsonOutput?: boolean;
  dryRun?: boolean;
  endpointUrl?: string;
}

export interface BlueprintRunResult {
  success: boolean;
  runId: string;
  action: BlueprintAction;
  output: string;
  exitCode: number;
}

function failResult(action: BlueprintAction, message: string): BlueprintRunResult {
  return { success: false, runId: "error", action, output: message, exitCode: 1 };
}

/**
 * Resolve the engine binary and args. Prefers the Rust nemoclaw-engine binary
 * (installed in bin/ or on PATH) and falls back to Python runner.py.
 */
function resolveEngine(
  blueprintPath: string,
): { program: string; baseArgs: string[]; useSubcommand: boolean } | null {
  // 1. Check for Rust binary alongside blueprint (bin/nemoclaw-engine)
  const rustBin = join(blueprintPath, "..", "bin", "nemoclaw-engine");
  if (existsSync(rustBin)) {
    return { program: rustBin, baseArgs: [], useSubcommand: true };
  }
  // 2. Check for Rust binary in nemoclaw-engine build output
  const rustRelease = join(blueprintPath, "..", "nemoclaw-engine", "target", "release", "nemoclaw-engine");
  if (existsSync(rustRelease)) {
    return { program: rustRelease, baseArgs: [], useSubcommand: true };
  }
  // 3. Fall back to Python runner.py
  const runnerPath = join(blueprintPath, "orchestrator", "runner.py");
  if (existsSync(runnerPath)) {
    return { program: "python3", baseArgs: [runnerPath], useSubcommand: false };
  }
  return null;
}

export async function execBlueprint(
  options: BlueprintRunOptions,
  logger: PluginLogger,
): Promise<BlueprintRunResult> {
  const engine = resolveEngine(options.blueprintPath);

  if (!engine) {
    const msg = `No blueprint engine found. Install nemoclaw-engine (Rust) or ensure orchestrator/runner.py exists.`;
    logger.error(msg);
    return failResult(options.action, msg);
  }

  // Build args: Rust uses subcommands, Python uses positional action.
  const args: string[] = [...engine.baseArgs];

  if (engine.useSubcommand) {
    // Rust CLI: nemoclaw-engine plan --profile default
    args.push(options.action, "--profile", options.profile);
    if (options.planPath) args.push("--plan", options.planPath);
    if (options.runId) args.push("--run-id", options.runId);
    if (options.dryRun) args.push("--dry-run");
    if (options.endpointUrl) args.push("--endpoint-url", options.endpointUrl);
  } else {
    // Python CLI: python3 runner.py plan --profile default
    args.push(options.action, "--profile", options.profile);
    if (options.jsonOutput) args.push("--json");
    if (options.planPath) args.push("--plan", options.planPath);
    if (options.runId) args.push("--run-id", options.runId);
    if (options.dryRun) args.push("--dry-run");
    if (options.endpointUrl) args.push("--endpoint-url", options.endpointUrl);
  }

  const engineLabel = engine.useSubcommand ? "Rust" : "Python";
  logger.info(`Running blueprint (${engineLabel}): ${options.action} (profile: ${options.profile})`);

  return new Promise((resolve) => {
    const chunks: string[] = [];
    const proc = spawn(engine.program, args, {
      cwd: options.blueprintPath,
      env: {
        ...process.env,
        NEMOCLAW_BLUEPRINT_PATH: options.blueprintPath,
        NEMOCLAW_ACTION: options.action,
      },
      stdio: ["pipe", "pipe", "pipe"],
    });

    proc.stdout.on("data", (data: Buffer) => {
      const line = data.toString();
      chunks.push(line);
    });

    proc.stderr.on("data", (data: Buffer) => {
      const line = data.toString().trim();
      if (line) logger.warn(line);
    });

    proc.on("close", (code) => {
      const output = chunks.join("");
      const runIdMatch = output.match(/^RUN_ID:(.+)$/m);
      resolve({
        success: code === 0,
        runId: runIdMatch?.[1] ?? "unknown",
        action: options.action,
        output,
        exitCode: code ?? 1,
      });
    });

    proc.on("error", (err) => {
      const msg = err.message.includes("ENOENT")
        ? `${engineLabel} engine not found. Install nemoclaw-engine or Python 3.11+.`
        : `Failed to start blueprint runner: ${err.message}`;
      logger.error(msg);
      resolve(failResult(options.action, msg));
    });
  });
}
