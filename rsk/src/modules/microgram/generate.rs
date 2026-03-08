use crate::modules::decision_engine::{DecisionNode, DecisionTree, Operator, Value};
use super::{Microgram, MicrogramTest};
use std::collections::HashMap;

/// Spec for generating a microgram
#[derive(Debug, Clone)]
pub struct MicrogramSpec {
    pub name: String,
    pub description: String,
    pub variable: String,
    pub operator: String,     // gt, gte, lt, lte, eq, is_null, is_not_null, matches
    pub threshold: Value,     // comparison value
    pub true_label: String,   // output key name when true
    pub true_value: Value,    // output value when true
    pub false_label: String,  // output key name when false
    pub false_value: Value,   // output value when false
}

impl MicrogramSpec {
    /// Generate a Microgram from this spec
    pub fn build(&self) -> Microgram {
        let operator = match self.operator.as_str() {
            "gt" => Operator::Gt,
            "gte" => Operator::Gte,
            "lt" => Operator::Lt,
            "lte" => Operator::Lte,
            "eq" => Operator::Eq,
            "is_null" => Operator::IsNull,
            "is_not_null" => Operator::IsNotNull,
            "matches" => Operator::Matches,
            _ => Operator::Eq, // fallback
        };

        let mut true_output = HashMap::new();
        true_output.insert(self.true_label.clone(), self.true_value.clone());

        let mut false_output = HashMap::new();
        false_output.insert(self.false_label.clone(), self.false_value.clone());

        let mut nodes = HashMap::new();
        nodes.insert(
            "check".to_string(),
            DecisionNode::Condition {
                variable: self.variable.clone(),
                operator,
                value: Some(self.threshold.clone()),
                true_next: "yes".to_string(),
                false_next: "no".to_string(),
            },
        );
        nodes.insert(
            "yes".to_string(),
            DecisionNode::Return {
                value: Value::Object(true_output.into_iter().collect()),
            },
        );
        nodes.insert(
            "no".to_string(),
            DecisionNode::Return {
                value: Value::Object(false_output.into_iter().collect()),
            },
        );

        // Auto-generate test cases from boundary analysis
        let tests = self.generate_tests();

        Microgram {
            name: self.name.clone(),
            description: self.description.clone(),
            version: "0.1.0".to_string(),
            tree: DecisionTree {
                start: "check".to_string(),
                nodes,
            },
            tests,
            interface: None,
            primitive_signature: None,
        }
    }

    /// Auto-generate boundary test cases
    fn generate_tests(&self) -> Vec<MicrogramTest> {
        let mut tests = Vec::new();

        match &self.threshold {
            Value::Int(n) => {
                let n = *n;
                // At threshold
                let mut at_input = HashMap::new();
                at_input.insert(self.variable.clone(), Value::Int(n));
                let at_expected = match self.operator.as_str() {
                    "gt" | "matches" => {
                        let mut m = HashMap::new();
                        m.insert(self.false_label.clone(), self.false_value.clone());
                        m
                    }
                    _ => {
                        let mut m = HashMap::new();
                        m.insert(self.true_label.clone(), self.true_value.clone());
                        m
                    }
                };
                tests.push(MicrogramTest { input: at_input, expect: at_expected });

                // Above threshold
                let mut above_input = HashMap::new();
                above_input.insert(self.variable.clone(), Value::Int(n + 1));
                let mut above_expected = HashMap::new();
                above_expected.insert(self.true_label.clone(), self.true_value.clone());
                tests.push(MicrogramTest { input: above_input, expect: above_expected });

                // Below threshold
                let mut below_input = HashMap::new();
                below_input.insert(self.variable.clone(), Value::Int(n - 1));
                let mut below_expected = HashMap::new();
                below_expected.insert(self.false_label.clone(), self.false_value.clone());
                tests.push(MicrogramTest { input: below_input, expect: below_expected });
            }
            Value::Float(f) => {
                let f = *f;
                let mut at_input = HashMap::new();
                at_input.insert(self.variable.clone(), Value::Float(f));
                let at_expected = match self.operator.as_str() {
                    "gt" => {
                        let mut m = HashMap::new();
                        m.insert(self.false_label.clone(), self.false_value.clone());
                        m
                    }
                    _ => {
                        let mut m = HashMap::new();
                        m.insert(self.true_label.clone(), self.true_value.clone());
                        m
                    }
                };
                tests.push(MicrogramTest { input: at_input, expect: at_expected });

                let mut above_input = HashMap::new();
                above_input.insert(self.variable.clone(), Value::Float(f + 1.0));
                let mut above_expected = HashMap::new();
                above_expected.insert(self.true_label.clone(), self.true_value.clone());
                tests.push(MicrogramTest { input: above_input, expect: above_expected });

                let mut below_input = HashMap::new();
                below_input.insert(self.variable.clone(), Value::Float(f - 1.0));
                let mut below_expected = HashMap::new();
                below_expected.insert(self.false_label.clone(), self.false_value.clone());
                tests.push(MicrogramTest { input: below_input, expect: below_expected });
            }
            _ => {
                // For string/bool comparisons, generate true/false pair
                let mut true_input = HashMap::new();
                true_input.insert(self.variable.clone(), self.threshold.clone());
                let mut true_expected = HashMap::new();
                true_expected.insert(self.true_label.clone(), self.true_value.clone());
                tests.push(MicrogramTest { input: true_input, expect: true_expected });
            }
        }

        tests
    }

    /// Serialize the generated microgram to YAML string
    pub fn to_yaml(&self) -> Result<String, String> {
        let mg = self.build();
        serde_yaml::to_string(&mg).map_err(|e| format!("YAML serialization error: {e}"))
    }
}
