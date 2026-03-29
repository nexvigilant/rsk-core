#!/usr/bin/env bash
set -euo pipefail

# rapid-relay.sh — Relay races across the rsk decision engine
# 10 chains fire in sequence. Each timed. Total measured.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
RSK="$PROJECT_DIR/target/release/rsk"
MG="$PROJECT_DIR/rsk/micrograms"
HG="$PROJECT_DIR/rsk/heligrams"

START=$(date +%s%N)
COUNT=0
PASS=0

run() {
  local label="$1"; shift
  COUNT=$((COUNT + 1))
  local RSTART=$(date +%s%N)
  local OUT
  OUT=$("$@" 2>/dev/null) || true
  local REND=$(date +%s%N)
  local MS=$(( (REND - RSTART) / 1000000 ))

  local SUMMARY
  SUMMARY=$(echo "$OUT" | python3 -c "
import sys,json
try:
  d=json.load(sys.stdin)
  o=d.get('final_output', d.get('output', d))
  keys=[k for k in o if k not in ('path','duration_us','name','success','mode','steps','total_duration_us')]
  print(' | '.join(f'{k}={o[k]}' for k in keys[:5]))
except:
  print('PARSE_FAIL')
" 2>/dev/null)

  if [ "$SUMMARY" != "PARSE_FAIL" ]; then
    PASS=$((PASS + 1))
    printf "  ✓ R%02d %-40s %4dms  %s\n" "$COUNT" "$label" "$MS" "$SUMMARY"
  else
    printf "  ✗ R%02d %-40s %4dms  FAIL\n" "$COUNT" "$label" "$MS"
  fi
}

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║  RELAY RACE — $(date +%H:%M:%S)                                        ║"
echo "╠════════════════════════════════════════════════════════════════╣"

run "prr→causality→naranjo→action" \
  "$RSK" mcg chain "prr-signal -> signal-to-causality -> naranjo-quick -> causality-to-action" \
  -d "$MG" --accumulate \
  -i '{"prr":4.2,"ror":5.1,"ic025":0.9,"chi_sq":35,"case_count":200,"on_label":false,"seriousness":"serious","is_expected":false,"naranjo_score":7}'

run "seriousness→deadline (fatal)" \
  "$RSK" mcg chain "case-seriousness -> seriousness-to-deadline" \
  -d "$MG" --accumulate \
  -i '{"seriousness":"serious","death":true,"hospitalization":true,"is_expected":false,"on_label":false}'

run "heligram PRR confirmed" \
  "$RSK" heligram run "$HG/prr-signal-forged.yaml" \
  -i '{"prr":3.8,"case_count":50,"notoriety_bias":false,"years_on_market":3,"channeling_bias":false}'

run "heligram PRR CONTESTED" \
  "$RSK" heligram run "$HG/prr-signal-forged.yaml" \
  -i '{"prr":3.2,"case_count":2,"notoriety_bias":true,"years_on_market":8,"channeling_bias":false}'

run "spanish cognate→conj→subjunctive" \
  "$RSK" mcg chain "spanish-cognate-classifier -> spanish-conjugation-tier -> spanish-subjunctive-gate" \
  -d "$MG" --accumulate \
  -i '{"latin_root":true,"ends_in_tion":true,"ends_in_al":false,"syllable_count":3,"verb_ending":"ar","is_regular":true,"is_reflexive":false,"is_factual":false,"expresses_doubt":true,"expresses_desire":false,"expresses_emotion":false,"is_command":false}'

run "autopilot pfizer" \
  "$RSK" mcg chain "pharma-safety-signal -> autopilot-pharma-sweep" \
  -d "$MG" --accumulate \
  -i '{"total_faers_reports":2365741,"top_reaction_count":195621,"total_reports":2365741,"death_count":79684,"new_signals":0,"company":"pfizer"}'

run "ecosystem-health-gate" \
  "$RSK" mcg run "$MG/ecosystem-health-gate.yaml" \
  -i '{"mcg_pass_rate":1.0,"hlg_pass_rate":1.0,"station_tools":1820,"wiring_pct":93,"prod_unwraps":0,"clippy_clean":true}'

run "flywheel-composite" \
  "$RSK" mcg run "$MG/flywheel/flywheel-composite.yaml" \
  -i '{"event_health":"HEALTHY","velocity_band":"GREEN","live_node_count":4,"staging_count":3,"crypto_health":"GREEN"}'

run "pharma h2h pfizer-vs-teva" \
  "$RSK" mcg run "$MG/pharma-head-to-head.yaml" \
  -i '{"company_a_reports":2365741,"company_b_reports":3488768,"company_a_deaths":79684,"company_b_deaths":100067,"company_a_signals":0,"company_b_signals":0}'

run "heligram 3-step PV chain" \
  "$RSK" heligram chain "prr-signal-helix -> case-seriousness-forged -> causality-to-action-forged" \
  -d "$HG" \
  -i '{"prr":5.0,"total_reports":100,"notoriety_bias":false,"seriousness":"serious","death":false,"hospitalization":true,"is_expected":false,"on_label":false,"causality":"PROBABLE"}'

END=$(date +%s%N)
TOTAL_MS=$(( (END - START) / 1000000 ))
AVG=$((TOTAL_MS / (COUNT > 0 ? COUNT : 1)))

echo "╠════════════════════════════════════════════════════════════════╣"
printf "║  RELAYS: %d/%d pass  |  TOTAL: %dms  |  AVG: %dms/relay          ║\n" "$PASS" "$COUNT" "$TOTAL_MS" "$AVG"
echo "╚════════════════════════════════════════════════════════════════╝"
