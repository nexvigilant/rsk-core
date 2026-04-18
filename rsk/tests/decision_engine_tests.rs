use rsk::modules::decision_engine::{
    DecisionContext, DecisionEngine, DecisionNode, DecisionTree, ExecutionResult, Operator, Value,
};
use std::collections::HashMap;

#[test]
fn test_complex_validation_logic() {
    // Scenario: Validate a user registration request
    // Logic:
    // 1. Check if username is present (fail if not)
    // 2. Check if age >= 18 (fail if not)
    // 3. Check if email contains "@" (fail if not)
    // 4. Return "Valid" if all pass

    let tree = DecisionTree {
        start: "check_username".to_string(),
        nodes: HashMap::from([
            (
                "check_username".to_string(),
                DecisionNode::Condition {
                    variable: "username".to_string(),
                    operator: Operator::IsNotNull,
                    value: None,
                    true_next: "check_age".to_string(),
                    false_next: "fail_username".to_string(),
                },
            ),
            (
                "check_age".to_string(),
                DecisionNode::Condition {
                    variable: "age".to_string(),
                    operator: Operator::Gte,
                    value: Some(Value::Int(18)),
                    true_next: "check_email".to_string(),
                    false_next: "fail_age".to_string(),
                },
            ),
            (
                "check_email".to_string(),
                DecisionNode::Condition {
                    variable: "email".to_string(),
                    operator: Operator::Contains,
                    value: Some(Value::String("@".to_string())),
                    true_next: "pass".to_string(),
                    false_next: "fail_email".to_string(),
                },
            ),
            (
                "pass".to_string(),
                DecisionNode::Return {
                    value: Value::String("Valid".to_string()),
                },
            ),
            (
                "fail_username".to_string(),
                DecisionNode::Return {
                    value: Value::String("Invalid: Username missing".to_string()),
                },
            ),
            (
                "fail_age".to_string(),
                DecisionNode::Return {
                    value: Value::String("Invalid: Underage".to_string()),
                },
            ),
            (
                "fail_email".to_string(),
                DecisionNode::Return {
                    value: Value::String("Invalid: Bad email".to_string()),
                },
            ),
        ]),
    };

    let engine = DecisionEngine::new(tree);

    // Case 1: Valid User
    let mut ctx = DecisionContext::new();
    ctx.set("username", Value::String("alice".to_string()));
    ctx.set("age", Value::Int(25));
    ctx.set("email", Value::String("alice@example.com".to_string()));

    match engine.execute(&mut ctx) {
        ExecutionResult::Value(v) => assert_eq!(v, Value::String("Valid".to_string())),
        _ => panic!("Expected valid result"),
    }
    assert_eq!(
        ctx.execution_path,
        vec!["check_username", "check_age", "check_email", "pass"]
    );

    // Case 2: Underage
    let mut ctx = DecisionContext::new();
    ctx.set("username", Value::String("bob".to_string()));
    ctx.set("age", Value::Int(16));
    ctx.set("email", Value::String("bob@example.com".to_string()));

    match engine.execute(&mut ctx) {
        ExecutionResult::Value(v) => assert_eq!(v, Value::String("Invalid: Underage".to_string())),
        _ => panic!("Expected invalid result"),
    }
    assert_eq!(
        ctx.execution_path,
        vec!["check_username", "check_age", "fail_age"]
    );

    // Case 3: Missing Username (IsNull check)
    let mut ctx = DecisionContext::new();
    ctx.set("age", Value::Int(30));
    ctx.set("email", Value::String("anon@example.com".to_string()));

    match engine.execute(&mut ctx) {
        ExecutionResult::Value(v) => {
            assert_eq!(v, Value::String("Invalid: Username missing".to_string()))
        }
        _ => panic!("Expected invalid result"),
    }
    assert_eq!(ctx.execution_path, vec!["check_username", "fail_username"]);
}
