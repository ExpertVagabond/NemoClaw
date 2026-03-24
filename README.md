# NemoClaw

[![License](https://img.shields.io/badge/License-Apache_2.0-blue)](LICENSE)
[![Status](https://img.shields.io/badge/status-alpha-orange)](docs/about/release-notes.md)
[![Node](https://img.shields.io/badge/node-%3E%3D20-brightgreen)](https://nodejs.org)
[![Security](https://img.shields.io/badge/security-report%20a%20vulnerability-red)](SECURITY.md)

Run [OpenClaw](https://openclaw.ai) agents inside [NVIDIA OpenShell](https://github.com/NVIDIA/OpenShell) sandboxes with managed inference routing to [NVIDIA Nemotron](https://build.nvidia.com) models.

> **Alpha** — available in early preview since March 16, 2026. APIs and behavior may change without notice.

---

## Architecture

NemoClaw is a four-component stack spanning four languages:

```
nemoclaw CLI (Node.js)          ← Host-side entry point
  │
  ├─ nemoclaw-engine (Rust)     ← Blueprint orchestrator (primary)
  ├─ runner.py (Python)         ← Blueprint orchestrator (fallback)
  │
  ├─ nemoclaw-orchestrator (Go) ← HTTP API for multi-sandbox management
  │
  └─ OpenShell sandbox          ← Isolated container running OpenClaw
       └─ Inference routed through OpenShell gateway
```

| Component | Language | Location | Role |
|-----------|----------|----------|------|
| **CLI** | Node.js | `bin/nemoclaw.js` | Host commands: onboard, connect, status, deploy, policy management |
| **Plugin** | TypeScript | `nemoclaw/` | OpenClaw plugin — blueprint execution, migration, in-sandbox commands |
| **Engine** | Rust | `nemoclaw-engine/` | Blueprint orchestrator — plan, apply, status, rollback. Compiled binary at `bin/nemoclaw-engine` |
| **Blueprint** | YAML + Python | `nemoclaw-blueprint/` | Declarative sandbox spec (`blueprint.yaml`), Python fallback runner, policy presets, migrations |
| **Orchestrator** | Go | `../nemoclaw-orchestrator/` | HTTP API (`:18800`) for multi-sandbox lifecycle and health probing |

### Engine Resolution

The TypeScript plugin resolves the blueprint engine with a Rust-first, Python-fallback strategy (see `nemoclaw/src/blueprint/exec.ts`):

1. Look for `bin/nemoclaw-engine` (compiled Rust binary)
2. Look for `nemoclaw-engine/target/release/nemoclaw-engine` (dev build)
3. Fall back to `python3 nemoclaw-blueprint/orchestrator/runner.py`

Both engines implement the same four actions (`plan`, `apply`, `status`, `rollback`) and communicate via the same stdout protocol (`RUN_ID:`, `PROGRESS:`).

---

## Quick Start

### Prerequisites

| Dependency | Version |
|------------|---------|
| Node.js | 20+ |
| npm | 10+ |
| Container runtime | Docker (Linux), Colima or Docker Desktop (macOS), Docker Desktop WSL (Windows) |
| [OpenShell](https://github.com/NVIDIA/OpenShell) | Installed and running |

| Resource | Minimum | Recommended |
|----------|---------|-------------|
| CPU | 4 vCPU | 4+ vCPU |
| RAM | 8 GB | 16 GB |
| Disk | 20 GB free | 40 GB free |

> For DGX Spark, follow the [Spark setup guide](spark-install.md) first.

### Install and Onboard

```bash
curl -fsSL https://www.nvidia.com/nemoclaw.sh | bash
```

The installer sets up Node.js if needed, then runs the onboard wizard which creates a sandbox, configures inference, and applies security policies.

When complete:

```
──────────────────────────────────────────────────
Sandbox      my-assistant (Landlock + seccomp + netns)
Model        nvidia/nemotron-3-super-120b-a12b (NVIDIA Cloud API)
──────────────────────────────────────────────────
Run:         nemoclaw my-assistant connect
Status:      nemoclaw my-assistant status
Logs:        nemoclaw my-assistant logs --follow
──────────────────────────────────────────────────
```

### Connect and Chat

```bash
# Connect to the sandbox shell
nemoclaw my-assistant connect

# Inside the sandbox — interactive TUI
openclaw tui

# Or send a single message via CLI
openclaw agent --agent main --local -m "hello" --session-id test
```

---

## CLI Reference

### Global Commands

```
nemoclaw onboard [--non-interactive]    Interactive setup wizard (recommended)
nemoclaw list                           List all registered sandboxes
nemoclaw deploy <instance>              Deploy to a Brev VM with GPU
nemoclaw start                          Start services (Telegram bridge, tunnel)
nemoclaw stop                           Stop all services
nemoclaw status                         Show sandboxes and service health
nemoclaw uninstall [--yes] [--keep-openshell] [--delete-models]
```

### Sandbox Commands

```
nemoclaw <name> connect                 Open an interactive shell
nemoclaw <name> status                  Sandbox health, inference, NIM status
nemoclaw <name> logs [--follow]         View sandbox logs
nemoclaw <name> policy-add              Interactively add a policy preset
nemoclaw <name> policy-list             List presets (● = applied)
nemoclaw <name> destroy                 Stop NIM + delete sandbox
```

### Engine Commands (Rust / Python)

```
nemoclaw-engine plan --profile default [--dry-run]
nemoclaw-engine apply --profile default [--plan <path>] [--endpoint-url <url>]
nemoclaw-engine status [--run-id <id>]
nemoclaw-engine rollback --run-id <id>
```

### OpenClaw Plugin (inside sandbox)

```
openclaw nemoclaw launch [--profile ...]
openclaw nemoclaw status
openclaw nemoclaw logs [-f]
```

---

## Inference

Inference calls from the agent never leave the sandbox directly. OpenShell intercepts and routes them through the configured provider.

| Profile | Provider | Model | Use Case |
|---------|----------|-------|----------|
| `default` | NVIDIA Cloud | `nemotron-3-super-120b-a12b` | Production — requires `NVIDIA_API_KEY` |
| `ncp` | NVIDIA NCP | `nemotron-3-super-120b-a12b` | Dynamic endpoint discovery |
| `nim-local` | OpenAI-compatible | `nemotron-3-super-120b-a12b` | Local NIM container |
| `vllm` | OpenAI-compatible | `nemotron-3-nano-30b-a3b` | Local vLLM |

Get an API key at [build.nvidia.com](https://build.nvidia.com). The onboard wizard prompts for this during setup. Local inference (Ollama, vLLM) is experimental.

---

## Security

Sandboxes start with a strict baseline enforced by OpenShell:

| Layer | Protection | Mutability |
|-------|-----------|------------|
| Network | Blocks unauthorized outbound connections | Hot-reloadable |
| Filesystem | Restricts access to `/sandbox` and `/tmp` | Locked at creation |
| Process | Blocks privilege escalation and dangerous syscalls | Locked at creation |
| Inference | Reroutes model API calls to controlled backends | Hot-reloadable |

Unauthorized network requests are blocked and surfaced in the TUI for operator approval.

### Policy Presets

Pre-built policy files in `nemoclaw-blueprint/policies/presets/`:

| Preset | Purpose |
|--------|---------|
| `discord` | Discord bot egress |
| `docker` | Docker daemon access |
| `huggingface` | HF Hub downloads |
| `jira` | Jira API access |
| `npm` | npm registry |
| `outlook` | Outlook/O365 |
| `pypi` | PyPI |
| `slack` | Slack API |
| `telegram` | Telegram bot API |

### MCP Server Policies

Pre-built MCP policies in `nemoclaw-blueprint/policies/mcp-servers/` for: Apple Automation, Coldstar, cPanel, Fulfil, iMessage, NVIDIA NIM, Ollama, Shopify, Solana.

---

## Building from Source

```bash
# Build the Rust engine + TS plugin
make install

# Build everything (engine + Go orchestrator + TS plugin)
make install-all

# Lint all languages
make check

# Build docs
make docs
```

The Rust engine compiles to `bin/nemoclaw-engine`. The Go orchestrator builds separately in `../nemoclaw-orchestrator/`.

---

## Uninstall

```bash
curl -fsSL https://raw.githubusercontent.com/NVIDIA/NemoClaw/refs/heads/main/uninstall.sh | bash
```

Removes sandboxes, gateway, providers, Docker images, local state, and the npm package. Does not remove Docker, Node.js, npm, or Ollama.

| Flag | Effect |
|------|--------|
| `--yes` | Skip confirmation |
| `--keep-openshell` | Leave `openshell` binary installed |
| `--delete-models` | Remove NemoClaw-pulled Ollama models |

---

## Project Structure

```
NemoClaw/
├── bin/
│   ├── nemoclaw.js              # Host CLI entry point
│   ├── nemoclaw-engine          # Compiled Rust binary
│   └── lib/                     # CLI modules (onboard, credentials, registry, nim, policies)
├── nemoclaw/                    # TypeScript OpenClaw plugin
│   ├── src/
│   │   ├── blueprint/           # exec.ts (engine resolution), fetch, resolve, verify, state
│   │   ├── commands/            # launch, connect, status, logs, migrate, onboard, eject
│   │   └── onboard/            # config, prompt, validate
│   └── openclaw.plugin.json
├── nemoclaw-engine/             # Rust blueprint orchestrator
│   └── src/
│       ├── main.rs              # CLI: plan, apply, status, rollback
│       ├── blueprint.rs         # Blueprint YAML parsing
│       ├── actions/             # plan, apply, status, rollback implementations
│       ├── protocol.rs          # RUN_ID / PROGRESS stdout protocol
│       ├── shell.rs             # Shell command execution
│       └── state.rs             # Run state persistence
├── nemoclaw-blueprint/
│   ├── blueprint.yaml           # Declarative sandbox + inference spec
│   ├── orchestrator/runner.py   # Python fallback engine
│   ├── policies/                # Base policies + presets + MCP server policies
│   └── migrations/
├── scripts/                     # Shell scripts (install, setup, services, tests)
├── test/                        # Node.js test suite (22 test files)
├── docs/                        # Sphinx documentation
├── Makefile                     # Build, lint, format, docs targets
├── package.json                 # npm package config (nemoclaw v0.1.0)
├── Dockerfile                   # Container build
└── install.sh                   # One-line installer
```

Related repository: [`nemoclaw-orchestrator`](https://github.com/ExpertVagabond/nemoclaw-orchestrator) — Go HTTP API for multi-sandbox management (port 18800).

---

## Documentation

- [Overview](https://docs.nvidia.com/nemoclaw/latest/about/overview.html)
- [How It Works](https://docs.nvidia.com/nemoclaw/latest/about/how-it-works.html)
- [Architecture](https://docs.nvidia.com/nemoclaw/latest/reference/architecture.html)
- [Inference Profiles](https://docs.nvidia.com/nemoclaw/latest/reference/inference-profiles.html)
- [Network Policies](https://docs.nvidia.com/nemoclaw/latest/reference/network-policies.html)
- [CLI Commands](https://docs.nvidia.com/nemoclaw/latest/reference/commands.html)
- [Troubleshooting](https://docs.nvidia.com/nemoclaw/latest/reference/troubleshooting.html)
- [Discord](https://discord.gg/XFpfPv9Uvx)

## License

[Apache License 2.0](LICENSE)
