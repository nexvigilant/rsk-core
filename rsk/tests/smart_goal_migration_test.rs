use rsk::modules::decision_engine::{
    DecisionContext, DecisionEngine, DecisionTree, ExecutionResult, Value,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn test_smart_goal_migration() {
    // 1. Load the YAML definition
    let yaml_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("skills")
        .join("smart-goal")
        .join("logic.yaml");

    let yaml_content = fs::read_to_string(&yaml_path).expect("Failed to read logic.yaml");

    let tree: DecisionTree =
        serde_yaml::from_str(&yaml_content).expect("Failed to parse DecisionTree");
    println!("Parsed Tree: {:?}", tree);

    let engine = DecisionEngine::new(tree);

    // Scenario 1: Perfect SMART Goal
    let mut ctx = DecisionContext::new();
    ctx.set(
        "goal_text",
        Value::String("Set a specific target to increase retention by 10%".to_string()),
    );
    ctx.set("resources_available", Value::Bool(true));

    match engine.execute(&mut ctx) {
        ExecutionResult::Value(val) => {
            if let Value::Object(map) = val {
                assert_eq!(map.get("status"), Some(&Value::String("valid".to_string())));
                assert_eq!(map.get("score"), Some(&Value::Int(100)));
            } else {
                panic!("Expected object result");
            }
        }
        other => panic!(
            "Execution failed or requested LLM: {:?}, Path: {:?}",
            other, ctx.execution_path
        ),
    }

    // Scenario 2: Vague Goal
    let mut ctx = DecisionContext::new();
    ctx.set("goal_text", Value::String("Do better".to_string()));
    ctx.set("resources_available", Value::Bool(true));

    match engine.execute(&mut ctx) {
        ExecutionResult::Value(val) => {
            if let Value::Object(map) = val {
                assert_eq!(
                    map.get("status"),
                    Some(&Value::String("invalid".to_string()))
                );
                assert_eq!(
                    map.get("message"),
                    Some(&Value::String("Goal lacks specificity".to_string()))
                );
            }
        }
        _ => panic!("Execution failed"),
    }

    // Scenario 3: Unmeasurable Goal
    let mut ctx = DecisionContext::new();
    ctx.set(
        "goal_text",
        Value::String("Improve target performance significantly".to_string()),
    );
    // "target" passes specificity check, but no numbers/metrics for measurability check

    match engine.execute(&mut ctx) {
        ExecutionResult::Value(val) => {
            if let Value::Object(map) = val {
                assert_eq!(
                    map.get("status"),
                    Some(&Value::String("invalid".to_string()))
                );
                assert_eq!(
                    map.get("message"),
                    Some(&Value::String(
                        "Goal is not measurable (missing numbers/metrics)".to_string()
                    ))
                );
            }
        }
        _ => panic!("Execution failed"),
    }
}
