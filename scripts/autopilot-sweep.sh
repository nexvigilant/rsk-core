#!/usr/bin/env bash
set -euo pipefail

# autopilot-sweep.sh — Sweep all pharma companies through the autopilot chain
#
# Usage:
#   ./scripts/autopilot-sweep.sh                    # Full sweep, JSON output
#   ./scripts/autopilot-sweep.sh --summary          # One-line-per-company summary
#   ./scripts/autopilot-sweep.sh --alerts-only      # Only show CRITICAL/HIGH
#
# Requires: rsk binary built, pharma_proxy.py accessible, python3 + jq

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

RSK="$PROJECT_DIR/target/release/rsk"
PROXY="$HOME/ferroforge/scripts/pharma_proxy.py"
MCG_DIR="$PROJECT_DIR/rsk/micrograms"
PRIOR_CACHE="$PROJECT_DIR/.autopilot-prior.json"

COMPANIES=(
  pfizer novartis roche merck abbvie bms jnj lilly
  gsk sanofi astrazeneca amgen gilead bayer novonordisk
  regeneron biogen moderna vertex alexion takeda boehringer
  teva astellas daiichi ucb jazz ipsen seagen incyte
  servier menarini lupin cspc celltrion samsungbioepis
  hengrui cipla drreddy sunpharma eisai
)

MODE="${1:---summary}"

# Load prior run cache for delta computation
declare -A PRIOR_REPORTS PRIOR_DEATHS
if [[ -f "$PRIOR_CACHE" ]]; then
  while IFS='|' read -r co pr pd; do
    PRIOR_REPORTS["$co"]="$pr"
    PRIOR_DEATHS["$co"]="$pd"
  done < <(python3 -c "
import json, sys
with open('$PRIOR_CACHE') as f:
    for co, v in json.load(f).items():
        print(f\"{co}|{v.get('total_reports',0)}|{v.get('death_count',0)}\")
" 2>/dev/null || true)
fi

echo "{"
echo "  \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
echo "  \"companies\": ["

FIRST=true
CRITICAL=0
HIGH=0
WATCH=0
ROUTINE=0
NEW_CACHE="{}"

for co in "${COMPANIES[@]}"; do
  # Fetch safety profile from pharma proxy — surface errors to log
  safety=$(echo "{\"tool\":\"get-safety-profile\",\"arguments\":{\"company_key\":\"$co\"}}" \
    | python3 "$PROXY" 2>"/tmp/autopilot-proxy-${co}.err") || {
    echo "  WARN: proxy failed for $co (see /tmp/autopilot-proxy-${co}.err)" >&2
    safety="{}"
  }

  # Single parse: extract all fields at once
  read -r total_reports death_count top_reaction new_signals < <(python3 -c "
import sys, json
d = json.load(sys.stdin)
total = d.get('total_reports', 0) or 0
reactions = d.get('top_reactions', [])
deaths = next((r['report_count'] for r in reactions if r.get('reaction','').upper() == 'DEATH'), 0)
top = reactions[0]['report_count'] if reactions else 0
# Count reactions with elevated reporting as proxy for disproportionality signals
# PRR proxy: reactions with >1% of total reports AND >100 absolute reports
signals = sum(1 for r in reactions if r.get('report_count', 0) > 100 and total > 0 and r.get('report_count', 0) / total > 0.01)
print(f'{total} {deaths} {top} {signals}')
" <<< "$safety" 2>/dev/null || echo "0 0 0 0")

  # Compute deltas from prior run
  prior_reports="${PRIOR_REPORTS[$co]:-0}"
  prior_deaths="${PRIOR_DEATHS[$co]:-0}"
  death_ratio="0"
  report_delta_pct="0"

  if [[ "$prior_deaths" -gt 0 ]] && [[ "$death_count" -gt 0 ]]; then
    death_ratio=$(python3 -c "print(round($death_count / $prior_deaths, 3))")
  fi
  if [[ "$prior_reports" -gt 0 ]] && [[ "$total_reports" -gt 0 ]]; then
    report_delta_pct=$(python3 -c "print(round(($total_reports - $prior_reports) / $prior_reports * 100, 1))")
  fi

  # Build chain input with pre-computed ratios
  chain_input=$(python3 -c "
import json
print(json.dumps({
    'total_faers_reports': $total_reports,
    'top_reaction_count': $top_reaction,
    'total_reports': $total_reports,
    'death_count': $death_count,
    'new_signals': $new_signals,
    'death_ratio': $death_ratio,
    'report_delta_pct': $report_delta_pct,
    'company': '$co'
}))")

  # Run autopilot chain (uses chain YAML definition)
  result=$($RSK mcg chain-test rsk/chains/autopilot-pharma-monitor.yaml 2>/dev/null || \
    $RSK mcg chain "pharma-safety-signal -> autopilot-pharma-sweep" \
      -d "$MCG_DIR" --accumulate \
      -i "$chain_input" 2>/dev/null) || {
    echo "  WARN: chain failed for $co" >&2
    continue
  }

  # Extract results — single parse, JSON-safe
  read -r alert action tier freq < <(python3 -c "
import sys, json
d = json.load(sys.stdin).get('final_output', {})
print(f\"{d.get('alert_level','UNKNOWN')} {d.get('action','unknown')} {d.get('signal_tier','unknown')} {d.get('monitoring_frequency','unknown')}\")
" <<< "$result" 2>/dev/null || echo "UNKNOWN unknown unknown unknown")

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
    printf "    %-15s  %8s reports  deaths=%6s  signals=%2s  delta=%.1f%%  tier=%-20s  alert=%-8s  action=%s\n" \
      "$co" "$total_reports" "$death_count" "$new_signals" "$report_delta_pct" "$tier" "$alert" "$action" >&2
  fi

  # JSON-safe output via python (no string interpolation breakage)
  $FIRST || echo ","
  FIRST=false
  python3 -c "
import json
print('    ' + json.dumps({
    'company': '$co',
    'total_reports': $total_reports,
    'death_count': $death_count,
    'new_signals': $new_signals,
    'death_ratio': $death_ratio,
    'report_delta_pct': $report_delta_pct,
    'tier': '$tier',
    'frequency': '$freq',
    'alert': '$alert',
    'action': '$action'
}))"

  # Cache current values for next run's delta computation
  NEW_CACHE=$(python3 -c "
import json
c = json.loads('$NEW_CACHE')
c['$co'] = {'total_reports': $total_reports, 'death_count': $death_count}
print(json.dumps(c))
")
done

echo ""
echo "  ],"
echo "  \"summary\": {\"critical\":$CRITICAL,\"high\":$HIGH,\"watch\":$WATCH,\"routine\":$ROUTINE,\"total\":${#COMPANIES[@]}}"
echo "}"

# Persist cache for next run
echo "$NEW_CACHE" > "$PRIOR_CACHE"
