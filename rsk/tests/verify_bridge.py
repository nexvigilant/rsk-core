import rsk
import json

# Sample Decision Tree YAML
tree_yaml = """
start: check_input
nodes:
  check_input:
    type: condition
    variable: score
    operator: gt
    value: 80
    true_next: pass
    false_next: fail
  pass:
    type: return
    value: "SUCCESS"
  fail:
    type: return
    value: "FAILURE"
"""

def test_bridge():
    print("Testing RSK Python Bridge...")
    
    # Test 1: Logic Execution
    inputs = {"score": 90}
    result = rsk.execute_logic(tree_yaml, inputs)
    print(f"Execution Result (score 90): {result['status']} -> {result.get('value')}")
    assert result['status'] == "success"
    assert result['value'] == "SUCCESS"
    
    inputs = {"score": 70}
    result = rsk.execute_logic(tree_yaml, inputs)
    print(f"Execution Result (score 70): {result['status']} -> {result.get('value')}")
    assert result['value'] == "FAILURE"

    # Test 2: Logic Generation
    skill_content = """---
name: test-skill
version: 1.0.0
compliance-level: diamond
---
## Machine Specification
### 6. INVARIANTS
- score must be positive
"""
    logic_yaml = rsk.generate_logic(skill_content)
    print("Generated Logic YAML:")
    print(logic_yaml)
    assert "start_node" in logic_yaml
    
    print("\n✅ Python Bridge Verified Successfully!")

if __name__ == "__main__":
    test_bridge()

