use rsk::modules::decision_engine::{
    DecisionContext, DecisionEngine, DecisionTree, ExecutionResult, Value,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn test_is_prime_full_migration() {
    let yaml_path = PathBuf::from("/home/matthew/.claude/skills/is-prime/logic.yaml");
    let yaml_content = fs::read_to_string(&yaml_path).expect("Failed to read logic.yaml");
    let tree: DecisionTree =
        serde_yaml::from_str(&yaml_content).expect("Failed to parse DecisionTree");
    let engine = DecisionEngine::new(tree);

    // Case 1: Prime Number
    let mut ctx = DecisionContext::new();
    ctx.set("n", Value::Int(17));
    match engine.execute(&mut ctx) {
        ExecutionResult::Value(val) => {
            if let Value::Object(map) = val {
                assert_eq!(
                    map.get("status"),
                    Some(&Value::String("success".to_string()))
                );
                // We don't have {{templating}} yet, but we'll check if the output is correct
            }
        }
        _ => panic!("Prime check failed"),
    }

    // Verify context has the result
    if let Some(Value::Object(res)) = ctx.get("result") {
        assert_eq!(res.get("is_prime"), Some(&Value::Bool(true)));
    } else {
        panic!("Missing result in context");
    }

    // Case 2: Non-Prime
    let mut ctx = DecisionContext::new();
    ctx.set("n", Value::Int(10));
    engine.execute(&mut ctx);
    if let Some(Value::Object(res)) = ctx.get("result") {
        assert_eq!(res.get("is_prime"), Some(&Value::Bool(false)));
    }

    // Case 3: Missing Input
    let mut ctx = DecisionContext::new();
    match engine.execute(&mut ctx) {
        ExecutionResult::Value(val) => {
            if let Value::Object(map) = val {
                assert_eq!(map.get("status"), Some(&Value::String("error".to_string())));
                assert_eq!(
                    map.get("message"),
                    Some(&Value::String("Input 'n' is missing".to_string()))
                );
            }
        }
        _ => panic!("Error handling failed"),
    }
}
