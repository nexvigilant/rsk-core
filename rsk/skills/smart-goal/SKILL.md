---
name: smart-goal
version: 1.0.0
compliance-level: diamond
---
# smart-goal
SMART goal validation — checks specificity, measurability, achievability.
## Machine Specification
### 1. INPUTS
- goal_text: string — the goal statement to validate
- resources_available: boolean — whether resources are available
### 2. OUTPUTS
- status: valid | invalid
- score: 0-100
- message: explanation
