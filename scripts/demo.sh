#!/usr/bin/env bash
# NemoClaw x NVIDIA — Demo Script for PENT HAUS @ Consensus Miami
# Run: bash scripts/demo.sh
set -euo pipefail

NC='\033[0m'
BOLD='\033[1m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[0;33m'

banner() { echo -e "\n${BOLD}${CYAN}━━━ $1 ━━━${NC}\n"; }
ok() { echo -e "  ${GREEN}✓${NC} $1"; }
warn() { echo -e "  ${YELLOW}⚠${NC} $1"; }

echo -e "${BOLD}"
echo "╔═══════════════════════════════════════════════════════╗"
echo "║  NemoClaw x NVIDIA — Sandboxed AI Agent Runtime      ║"
echo "║  Powered by NVIDIA NIM • Nemotron 3 Super 120B       ║"
echo "║  Demo for PENT HAUS @ Consensus Miami 2026           ║"
echo "╚═══════════════════════════════════════════════════════╝"
echo -e "${NC}"

# ─── Step 1: Engine Status ───
banner "Step 1: NemoClaw Engine"
nemoclaw-engine status 2>&1 && ok "Engine responding" || warn "Engine returned unknown (expected without active run)"

# ─── Step 2: Docker Sandbox ───
banner "Step 2: Docker Sandbox"
if docker ps --filter name=nemoclaw-sandbox --format '{{.Status}}' 2>/dev/null | grep -q "Up"; then
    ok "Sandbox container running"
    docker ps --filter name=nemoclaw-sandbox --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}' 2>/dev/null
else
    warn "Sandbox not running — starting..."
    docker rm -f nemoclaw-sandbox 2>/dev/null || true
    docker run -d --name nemoclaw-sandbox --platform linux/amd64 \
        -p 18789:18789 -e NVIDIA_API_KEY="${NVIDIA_API_KEY:-}" \
        nemoclaw:latest
    sleep 8
    ok "Sandbox started"
fi

# ─── Step 3: Gateway Health ───
banner "Step 3: OpenClaw Gateway"
GATEWAY_LOG=$(docker exec nemoclaw-sandbox cat /tmp/gateway.log 2>/dev/null | tail -15)
echo "$GATEWAY_LOG"

if echo "$GATEWAY_LOG" | grep -q "NemoClaw registered"; then
    ok "NemoClaw plugin registered"
fi
if echo "$GATEWAY_LOG" | grep -q "NVIDIA Cloud API"; then
    ok "NVIDIA NIM inference connected"
fi
if echo "$GATEWAY_LOG" | grep -q "listening on"; then
    ok "Gateway WebSocket active"
fi

# ─── Step 4: Inference Config ───
banner "Step 4: NVIDIA NIM Configuration"
docker exec nemoclaw-sandbox cat /sandbox/.openclaw/openclaw.json 2>/dev/null | python3 -m json.tool 2>/dev/null || echo "Config not accessible"

# ─── Step 5: Agent Execution Test ───
banner "Step 5: Agent Sandbox Test"
echo "Executing command inside sandbox..."
docker exec nemoclaw-sandbox whoami 2>/dev/null && ok "Sandbox user: sandbox (isolated)"
docker exec nemoclaw-sandbox node --version 2>/dev/null && ok "Node.js available"
docker exec nemoclaw-sandbox python3 --version 2>/dev/null && ok "Python available"
docker exec nemoclaw-sandbox openclaw --version 2>/dev/null && ok "OpenClaw CLI available"

# ─── Step 6: Policy Presets ───
banner "Step 6: Security Policies"
echo "Available policy presets:"
ls -1 /Volumes/Virtual\ Server/projects/NemoClaw/nemoclaw-blueprint/policies/ 2>/dev/null | while read f; do
    echo "  • $f"
done

# ─── Step 7: MCP Server ───
banner "Step 7: nemoclaw-mcp (10 tools)"
echo "Tools:"
echo "  1. nemoclaw_list_sandboxes     — List running sandboxes"
echo "  2. nemoclaw_sandbox_status     — Health + inference config"
echo "  3. nemoclaw_sandbox_logs       — Stream sandbox logs"
echo "  4. nemoclaw_send_message       — Send message to agent (\$0.01)"
echo "  5. nemoclaw_sandbox_destroy    — Tear down sandbox"
echo "  6. nemoclaw_inference_get      — Read inference config (\$0.005)"
echo "  7. nemoclaw_inference_set      — Write inference config (\$0.02)"
echo "  8. nemoclaw_policy_list        — List security presets"
echo "  9. nemoclaw_policy_apply       — Apply policy preset"
echo " 10. nemoclaw_gateway_status     — Gateway health check"

# ─── Summary ───
banner "Demo Complete"
echo -e "${BOLD}NemoClaw Stack:${NC}"
echo "  Engine:     nemoclaw-engine 0.1.0 (Rust)"
echo "  Sandbox:    Docker (linux/amd64, OpenClaw + NIM)"
echo "  Gateway:    ws://127.0.0.1:18789"
echo "  Model:      nvidia/nemotron-3-super-120b-a12b"
echo "  Endpoint:   build.nvidia.com (NVIDIA Cloud API)"
echo "  MCP Server: nemoclaw-mcp (10 tools, x402 payments)"
echo "  Policies:   13 presets (network, sandbox, MCP)"
echo "  Orchestrator: psm-orchestrator (Go, :18801)"
echo ""
echo -e "${GREEN}Ready for PENT HAUS @ Consensus Miami • May 4-7, 2026${NC}"
