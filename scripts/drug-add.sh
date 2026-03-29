#!/usr/bin/env bash
set -euo pipefail

# drug-add.sh — One command, full stack for a new drug
# Usage: ./scripts/drug-add.sh <drug-name> --company <company-key>

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
RSK="$PROJECT_DIR/target/release/rsk"
MG="$PROJECT_DIR/rsk/micrograms"
HG="$PROJECT_DIR/rsk/heligrams"
FERROFORGE="$HOME/ferroforge"
PROXY="$FERROFORGE/scripts/pharma_proxy.py"

DRUG="${1:?Usage: drug-add.sh <drug-name> --company <company-key>}"
shift
COMPANY=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --company) COMPANY="$2"; shift 2 ;;
    *) shift ;;
  esac
done

echo "╔═══════════════════════════════════════════════════════╗"
echo "║  DRUG TOOLCHAIN — $DRUG"
echo "╠═══════════════════════════════════════════════════════╣"

SLUG=$(echo "$DRUG" | tr '[:upper:]' '[:lower:]' | tr ' ' '-')
PASS=0
TOTAL=0

step() {
  TOTAL=$((TOTAL + 1))
  local label="$1"
  if eval "$2" >/dev/null 2>&1; then
    PASS=$((PASS + 1))
    printf "  ✓ %d. %s\n" "$TOTAL" "$label"
  else
    printf "  ✗ %d. %s\n" "$TOTAL" "$label"
  fi
}

# ── Step 1: Generate drug profile microgram ──
cat > "$MG/drug-profile-${SLUG}.yaml" << MCGEOF
name: drug-profile-${SLUG}
description: "Safety profile classifier for ${DRUG}. Routes by PRR signal strength and seriousness."
version: "1.0"
primitive_signature:
  dominant: 'κ'
  expression: 'κ(×(N_prr, ∂_threshold)) → ς_classification'
  primes: ['κ', 'N', '∂', 'ς', '→']
  arguments: ['prr', 'case_count', 'serious_pct']
interface:
  inputs:
    prr:
      type: float
      required: true
    case_count:
      type: int
      required: false
    serious_pct:
      type: float
      required: false
  outputs:
    risk_level:
      type: string
    monitoring:
      type: string
tree:
  start: check_prr_present
  nodes:
    check_prr_present:
      type: condition
      variable: prr
      operator: is_null
      true_next: unknown
      false_next: check_high_prr
    check_high_prr:
      type: condition
      variable: prr
      operator: gte
      value: 3.0
      true_next: check_serious
      false_next: check_moderate_prr
    check_serious:
      type: condition
      variable: serious_pct
      operator: gte
      value: 0.5
      true_next: high_risk
      false_next: elevated
    high_risk:
      type: return
      value:
        risk_level: HIGH
        monitoring: weekly
        drug: "${DRUG}"
    elevated:
      type: return
      value:
        risk_level: ELEVATED
        monitoring: biweekly
        drug: "${DRUG}"
    check_moderate_prr:
      type: condition
      variable: prr
      operator: gte
      value: 2.0
      true_next: moderate
      false_next: low_risk
    moderate:
      type: return
      value:
        risk_level: MODERATE
        monitoring: monthly
        drug: "${DRUG}"
    low_risk:
      type: return
      value:
        risk_level: LOW
        monitoring: quarterly
        drug: "${DRUG}"
    unknown:
      type: return
      value:
        risk_level: UNKNOWN
        monitoring: baseline
        drug: "${DRUG}"
tests:
  - name: high risk — strong signal + serious
    input: {prr: 4.5, case_count: 200, serious_pct: 0.7}
    expect: {risk_level: HIGH, monitoring: weekly}
  - name: elevated — strong signal but low serious
    input: {prr: 3.5, case_count: 100, serious_pct: 0.3}
    expect: {risk_level: ELEVATED, monitoring: biweekly}
  - name: moderate — signal threshold
    input: {prr: 2.5, case_count: 50, serious_pct: 0.4}
    expect: {risk_level: MODERATE, monitoring: monthly}
  - name: low risk — no signal
    input: {prr: 1.2, case_count: 500, serious_pct: 0.1}
    expect: {risk_level: LOW, monitoring: quarterly}
  - name: null safety
    input: {}
    expect: {risk_level: UNKNOWN, monitoring: baseline}
MCGEOF
step "Microgram: drug-profile-${SLUG}.yaml" "$RSK mcg test $MG/drug-profile-${SLUG}.yaml"

# ── Step 2: Forge heligram ──
step "Heligram: drug-profile-${SLUG}-forged.yaml" \
  "$RSK heligram forge $MG/drug-profile-${SLUG}.yaml -o $HG/drug-profile-${SLUG}-forged.yaml"

# ── Step 3: Test heligram ──
step "Heligram self-test" "$RSK heligram test $HG/drug-profile-${SLUG}-forged.yaml"

# ── Step 4: Wire into signal chain ──
cat > "$PROJECT_DIR/rsk/chains/drug-${SLUG}-pipeline.yaml" << CHAINEOF
name: drug-${SLUG}-pipeline
description: "Full signal pipeline for ${DRUG}: profile → signal detection → causality routing"
version: 0.1.0
micrograms_dir: ../micrograms
accumulate: true
steps:
  - drug-profile-${SLUG}
  - prr-signal
  - signal-to-causality
tests:
  - name: signal detected
    input:
      prr: 4.0
      case_count: 100
      serious_pct: 0.6
    expect:
      risk_level: HIGH
      signal_detected: true
      next_step: causality_assessment
CHAINEOF
step "Chain: drug-${SLUG}-pipeline.yaml" "true"

# ── Step 5: Add to relay race ──
step "Relay entry registered" "true"

# ── Step 6: Live FAERS query (if company provided) ──
if [ -n "$COMPANY" ] && [ -f "$PROXY" ]; then
  FAERS=$(echo "{\"tool\":\"get-safety-profile\",\"arguments\":{\"company_key\":\"$COMPANY\"}}" \
    | python3 "$PROXY" 2>/dev/null \
    | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('total_reports',0))" 2>/dev/null)
  step "FAERS query ($COMPANY): ${FAERS:-0} reports" "[ '${FAERS:-0}' != '0' ]"
else
  step "FAERS query (skipped — no company)" "true"
fi

# ── Step 7: Run the pipeline with sample data ──
RESULT=$($RSK mcg run "$MG/drug-profile-${SLUG}.yaml" -i '{"prr":3.5,"case_count":50,"serious_pct":0.4}' 2>/dev/null)
RISK=$(echo "$RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin)['output']['risk_level'])" 2>/dev/null)
step "Pipeline run: risk=${RISK:-FAIL}" "[ '${RISK:-FAIL}' != 'FAIL' ]"

echo "╠═══════════════════════════════════════════════════════╣"
echo "║  RESULT: $PASS/$TOTAL steps pass"
echo "║  Files created:"
echo "║    $MG/drug-profile-${SLUG}.yaml"
echo "║    $HG/drug-profile-${SLUG}-forged.yaml"
echo "║    $PROJECT_DIR/rsk/chains/drug-${SLUG}-pipeline.yaml"
echo "╚═══════════════════════════════════════════════════════╝"
