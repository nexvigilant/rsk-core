---
name: severity-scorer
version: 1.0.0
compliance-level: diamond
---
# severity-scorer
Score severity from detection confidence and context flags.
## Machine Specification
### 1. INPUTS
- confidence: float (0.0-1.0)
- is_critical_path: boolean
### 2. OUTPUTS
- severity: LOW | MEDIUM | HIGH | CRITICAL
- score: 0-100
