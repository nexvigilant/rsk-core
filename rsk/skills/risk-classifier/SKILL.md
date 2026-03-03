---
name: risk-classifier
version: 1.0.0
compliance-level: diamond
---
# risk-classifier
Classify risk action from a numeric score.
## Machine Specification
### 1. INPUTS
- score: integer (0-100)
### 2. OUTPUTS
- action: ACCEPT | MONITOR | MITIGATE | ESCALATE
- rationale: explanation
