use rsk::{DecisionContext, DecisionEngine, DecisionTree, ExecutionResult, Value};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_strategy_engine_full_flow() {
    let yaml_path = PathBuf::from("/home/matthew/.claude/skills/strategy-engine/logic.yaml");
    let yaml_content = fs::read_to_string(&yaml_path).expect("Failed to read logic.yaml");
    let tree: DecisionTree =
        serde_yaml::from_str(&yaml_content).expect("Failed to parse DecisionTree");

    let mut ctx = DecisionContext::new();

    // Simulate data gathered in previous phases
    let mut strategy_data = HashMap::new();
    strategy_data.insert(
        "fields".to_string(),
        Value::Array(vec![Value::Object(HashMap::from([
            ("id".to_string(), Value::String("Pharma-Safety".to_string())),
            ("market_size".to_string(), Value::Float(500.0)),
            ("growth_rate".to_string(), Value::Float(0.12)),
            ("capability_fit".to_string(), Value::Float(0.85)),
            ("competitive_intensity".to_string(), Value::Float(0.3)),
        ]))]),
    );
    strategy_data.insert(
        "tactics".to_string(),
        Value::Array(vec![Value::Object(HashMap::from([
            ("id".to_string(), Value::String("AI-Automation".to_string())),
            ("differentiation".to_string(), Value::Float(0.95)),
            ("cost_advantage".to_string(), Value::Float(0.4)),
            ("execution_risk".to_string(), Value::Float(0.25)),
        ]))]),
    );

    ctx.set("strategy_data", Value::Object(strategy_data));
    ctx.set("input", Value::String("Initial context".to_string()));

    // We can't easily test the LLM fallback steps in a unit test,
    // but we can jump straight to the intrinsic optimization

    // Create a modified tree for testing that starts at exponential_optimization
    let mut test_tree = tree.clone();
    test_tree.start = "exponential_optimization".to_string();
    let test_engine = DecisionEngine::new(test_tree);

    match test_engine.execute(&mut ctx) {
        ExecutionResult::Value(val) => {
            if let Value::Object(map) = val {
                assert_eq!(
                    map.get("status"),
                    Some(&Value::String("success".to_string()))
                );
                println!("Winner: {:?}", map.get("optimal_strategy"));
            }
        }
        other => panic!("Strategy optimization failed: {:?}", other),
    }

    // Verify context has optimal paths
    assert!(ctx.get("optimal_paths").is_some());
}
