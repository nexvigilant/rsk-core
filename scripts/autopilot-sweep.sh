#!/usr/bin/env bash
set -euo pipefail

# autopilot-sweep.sh — Sweep all 15 pharma companies through the autopilot chain
#
# Usage:
#   ./scripts/autopilot-sweep.sh                    # Full sweep, JSON output
#   ./scripts/autopilot-sweep.sh --summary          # One-line-per-company summary
#   ./scripts/autopilot-sweep.sh --alerts-only      # Only show CRITICAL/HIGH
#
# Requires: rsk binary built, pharma_proxy.py accessible

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

RSK="$PROJECT_DIR/target/release/rsk"
PROXY="$HOME/ferroforge/scripts/pharma_proxy.py"
MCG_DIR="$PROJECT_DIR/rsk/micrograms"

COMPANIES=(
  pfizer novartis roche merck abbvie bms jnj lilly
  gsk sanofi astrazeneca amgen gilead bayer novonordisk
)

MODE="${1:---summary}"

echo "{"
echo "  \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
echo "  \"companies\": ["

FIRST=true
CRITICAL=0
HIGH=0
WATCH=0
ROUTINE=0

for co in "${COMPANIES[@]}"; do
  # Fetch safety profile from pharma proxy
  safety=$(echo "{\"tool\":\"get-safety-profile\",\"arguments\":{\"company_key\":\"$co\"}}" \
    | python3 "$PROXY" 2>/dev/null)

  total_reports=$(echo "$safety" | python3 -c "import sys,json; print(json.load(sys.stdin).get('total_reports',0))" 2>/dev/null || echo "0")

  # Extract death count from top reactions
  death_count=$(echo "$safety" | python3 -c "
import sys,json
d = json.load(sys.stdin)
deaths = [r['report_count'] for r in d.get('top_reactions',[]) if r.get('reaction','').upper() == 'DEATH']
print(deaths[0] if deaths else 0)
" 2>/dev/null || echo "0")

  top_reaction=$(echo "$safety" | python3 -c "
import sys,json
d = json.load(sys.stdin)
r = d.get('top_reactions',[])
print(r[0]['report_count'] if r else 0)
" 2>/dev/null || echo "0")

  # Run autopilot chain
  chain_input="{\"total_faers_reports\":$total_reports,\"top_reaction_count\":$top_reaction,\"total_reports\":$total_reports,\"death_count\":$death_count,\"new_signals\":0,\"company\":\"$co\"}"

  result=$($RSK mcg chain "pharma-safety-signal -> autopilot-pharma-sweep" \
    -d "$MCG_DIR" --accumulate \
    -i "$chain_input" 2>/dev/null)

  alert=$(echo "$result" | python3 -c "import sys,json; print(json.load(sys.stdin)['final_output'].get('alert_level','UNKNOWN'))" 2>/dev/null)
  action=$(echo "$result" | python3 -c "import sys,json; print(json.load(sys.stdin)['final_output'].get('action','unknown'))" 2>/dev/null)
  tier=$(echo "$result" | python3 -c "import sys,json; print(json.load(sys.stdin)['final_output'].get('signal_tier','unknown'))" 2>/dev/null)
  freq=$(echo "$result" | python3 -c "import sys,json; print(json.load(sys.stdin)['final_output'].get('monitoring_frequency','unknown'))" 2>/dev/null)

  # Count alerts
  case "$alert" in
    CRITICAL) CRITICAL=$((CRITICAL+1)) ;;
    HIGH) HIGH=$((HIGH+1)) ;;
    WATCH) WATCH=$((WATCH+1)) ;;
    ROUTINE) ROUTINE=$((ROUTINE+1)) ;;
  esac

  # Filter for alerts-only mode
  if [[ "$MODE" == "--alerts-only" ]] && [[ "$alert" != "CRITICAL" ]] && [[ "$alert" != "HIGH" ]]; then
    continue
  fi

  if [[ "$MODE" == "--summary" ]] || [[ "$MODE" == "--alerts-only" ]]; then
    printf "    %-15s  %8s reports  deaths=%6s  tier=%-20s  alert=%-8s  action=%s\n" \
      "$co" "$total_reports" "$death_count" "$tier" "$alert" "$action" >&2
  fi

  $FIRST || echo ","
  FIRST=false
  echo "    {\"company\":\"$co\",\"total_reports\":$total_reports,\"death_count\":$death_count,\"tier\":\"$tier\",\"frequency\":\"$freq\",\"alert\":\"$alert\",\"action\":\"$action\"}"
done

echo ""
echo "  ],"
echo "  \"summary\": {\"critical\":$CRITICAL,\"high\":$HIGH,\"watch\":$WATCH,\"routine\":$ROUTINE,\"total\":${#COMPANIES[@]}}"
echo "}"
