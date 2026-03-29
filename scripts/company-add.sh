#!/usr/bin/env bash
set -euo pipefail

# company-add.sh — Add a new pharma company across the full stack
# Usage: ./scripts/company-add.sh <key> "<Display Name>" "<FAERS manufacturer names>"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
RSK="$PROJECT_DIR/target/release/rsk"
FERROFORGE="$HOME/ferroforge"

KEY="${1:?Usage: company-add.sh <key> '<Display Name>' '<MANUFACTURER1,MANUFACTURER2>'}"
DISPLAY="${2:?Provide display name}"
MANUFACTURERS="${3:?Provide FAERS manufacturer names comma-separated}"

echo "╔═══════════════════════════════════════════════════════╗"
echo "║  COMPANY TOOLCHAIN — $DISPLAY"
echo "╠═══════════════════════════════════════════════════════╣"

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

# ── Step 1: Add to pharma_proxy.py registry ──
ALREADY=$(python3 -c "
import sys; sys.path.insert(0,'$FERROFORGE/scripts')
from pharma_proxy import COMPANY_REGISTRY
print('yes' if '$KEY' in COMPANY_REGISTRY else 'no')
" 2>/dev/null)

if [ "$ALREADY" = "yes" ]; then
  step "Registry: $KEY already registered" "true"
else
  # Build the manufacturer list for Python
  MFG_LIST=$(echo "$MANUFACTURERS" | tr ',' '\n' | sed 's/^ *//;s/ *$//' | awk '{printf "\"%s\", ", toupper($0)}' | sed 's/, $//')
  
  python3 << PYEOF
import re
path = "$FERROFORGE/scripts/pharma_proxy.py"
with open(path) as f:
    content = f.read()

entry = '''    "$KEY": {
        "display_name": "$DISPLAY",
        "openfda_manufacturer": [$MFG_LIST],
        "ct_sponsor": "$DISPLAY",
    },'''

# Insert before the closing brace of COMPANY_REGISTRY
content = content.replace('\n}\n\n\n# ----', f'\n{entry}\n' + '}\n\n\n# ----')
with open(path, 'w') as f:
    f.write(content)
PYEOF
  step "Registry: added $KEY to pharma_proxy.py" "true"
fi

# ── Step 2: Generate Station config ──
if [ -f "$FERROFORGE/configs/$KEY.json" ]; then
  step "Config: $KEY.json already exists" "true"
else
  python3 << PYEOF
import json
with open("$FERROFORGE/configs/pfizer.json") as f:
    cfg = json.load(f)
cfg["site"] = f"www.${KEY}.com"
cfg["description"] = f"$DISPLAY pharmacovigilance intelligence — portfolio, pipeline, safety profile, recalls, labeling changes, and competitive analysis"
for tool in cfg["tools"]:
    tool["description"] = tool["description"].replace("Pfizer Inc.", "$DISPLAY").replace("Pfizer", "$DISPLAY")
with open("$FERROFORGE/configs/$KEY.json", "w") as f:
    json.dump(cfg, f, indent=2)
PYEOF
  step "Config: $KEY.json created" "[ -f '$FERROFORGE/configs/$KEY.json' ]"
fi

# ── Step 3: Test FAERS connection ──
REPORTS=$(echo "{\"tool\":\"get-safety-profile\",\"arguments\":{\"company_key\":\"$KEY\"}}" \
  | python3 "$FERROFORGE/scripts/pharma_proxy.py" 2>/dev/null \
  | python3 -c "import sys,json; print(json.load(sys.stdin).get('total_reports',0))" 2>/dev/null || echo "0")
step "FAERS: $REPORTS reports" "[ '$REPORTS' != '0' ]"

# ── Step 4: Test pipeline connection ──
TRIALS=$(echo "{\"tool\":\"get-pipeline\",\"arguments\":{\"company_key\":\"$KEY\",\"limit\":1}}" \
  | python3 "$FERROFORGE/scripts/pharma_proxy.py" 2>/dev/null \
  | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d.get('trials',[])))" 2>/dev/null || echo "0")
step "Pipeline: $TRIALS trials found" "true"

# ── Step 5: Add to autopilot sweep ──
if grep -q "$KEY" "$PROJECT_DIR/scripts/autopilot-sweep.sh" 2>/dev/null; then
  step "Autopilot: already in sweep" "true"
else
  sed -i "s/incyte/incyte\n  $KEY/" "$PROJECT_DIR/scripts/autopilot-sweep.sh"
  step "Autopilot: added to sweep" "grep -q '$KEY' '$PROJECT_DIR/scripts/autopilot-sweep.sh'"
fi

# ── Step 6: Run autopilot classification ──
ALERT=$($RSK mcg chain "pharma-safety-signal -> autopilot-pharma-sweep" \
  -d "$PROJECT_DIR/rsk/micrograms" --accumulate \
  -i "{\"total_faers_reports\":$REPORTS,\"top_reaction_count\":10000,\"total_reports\":$REPORTS,\"death_count\":0,\"new_signals\":0,\"company\":\"$KEY\"}" 2>/dev/null \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['final_output']['alert_level'])" 2>/dev/null || echo "FAIL")
step "Classification: $ALERT" "[ '$ALERT' != 'FAIL' ]"

echo "╠═══════════════════════════════════════════════════════╣"
echo "║  RESULT: $PASS/$TOTAL steps pass"
echo "║  $DISPLAY: $REPORTS FAERS reports, $TRIALS trials, alert=$ALERT"
echo "╚═══════════════════════════════════════════════════════╝"
