use crate::modules::decision_engine::Value;
use std::collections::HashMap;

pub fn intrinsic_is_prime(input: Value) -> Value {
    let mut variables = HashMap::new();
    if let Value::Object(map) = input {
        variables = map;
    }

    let var_val = variables.get(" n").and_then(|v| v.as_f64()).unwrap_or(0.0);
    if var_val == 0.0 {
        let var_val = variables.get(" n").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if var_val >= -1000000000.0 {
            return Value::Null;
        } else {
            return {
                let mut m = HashMap::new();
                m.insert(
                    "message".to_string(),
                    Value::String("Input 'n' must be an integer".to_string()),
                );
                m.insert("status".to_string(), Value::String("error".to_string()));
                Value::Object(m)
            };
        }
    } else {
        return {
            let mut m = HashMap::new();
            m.insert("status".to_string(), Value::String("error".to_string()));
            m.insert(
                "message".to_string(),
                Value::String("Input 'n' is missing".to_string()),
            );
            Value::Object(m)
        };
    }
}
