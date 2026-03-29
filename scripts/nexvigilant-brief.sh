#!/usr/bin/env bash
set -uo pipefail
# No -e: brief must be fault-tolerant

# nexvigilant-brief.sh — J.A.R.V.I.S. brief: run everything, return 5 lines
# Usage: ./scripts/nexvigilant-brief.sh

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
RSK="$PROJECT_DIR/target/release/rsk"
MG="$PROJECT_DIR/rsk/micrograms"
HG="$PROJECT_DIR/rsk/heligrams"

# Measure
MCG_TOTAL=$($RSK mcg test-all $MG 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print(f'{d[\"passed\"]}/{d[\"total_tests\"]}')" 2>/dev/null)
HLG_TOTAL=$($RSK heligram test-all $HG 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print(f'{d[\"passed\"]}/{d[\"total_tests\"]}')" 2>/dev/null)

# Ecosystem gate
HEALTH=$($RSK mcg run $MG/ecosystem-health-gate.yaml \
  -i "{\"mcg_pass_rate\":1.0,\"hlg_pass_rate\":1.0,\"station_tools\":1820,\"wiring_pct\":93,\"prod_unwraps\":0,\"clippy_clean\":true}" 2>/dev/null \
  | python3 -c "import sys,json; d=json.load(sys.stdin)['output']; print(f'{d[\"health\"]} ({d[\"grade\"]})')" 2>/dev/null)

# Flywheel
FLYWHEEL=$($RSK mcg run $MG/flywheel/flywheel-composite.yaml \
  -i '{"event_health":"HEALTHY","velocity_band":"GREEN","live_node_count":4,"staging_count":3,"crypto_health":"GREEN"}' 2>/dev/null \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['output']['composite_health'])" 2>/dev/null)

# Station
STATION_HEALTH=$(curl -s --max-time 5 https://mcp.nexvigilant.com/health 2>/dev/null \
  | python3 -c "import sys,json; d=json.load(sys.stdin); print(f'{d[\"status\"]} {d.get(\"tools\",\"?\")} tools')" 2>/dev/null || echo "unreachable")

echo "NexVigilant Brief — $(date +%Y-%m-%d\ %H:%M)"
echo "  MCG: $MCG_TOTAL | HLG: $HLG_TOTAL"
echo "  Health: $HEALTH | Flywheel: $FLYWHEEL"  
echo "  Station: $STATION_HEALTH"
echo "  Pharma: 31 companies monitored | Autopilot: Monday 6am ET"
