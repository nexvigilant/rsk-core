use super::load_all;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct MatrixCell {
    pub runner: String,
    pub test_from: String,
    pub total: usize,
    pub executed: usize, // ran without error
    pub matched: usize,  // output matched expected
}

#[derive(Debug, Clone, Serialize)]
pub struct MatrixResult {
    pub cells: Vec<MatrixCell>,
    pub total_runs: usize,
    pub cross_matches: usize, // unexpected matches between different micrograms
}

/// Run every microgram against every other microgram's test inputs
pub fn matrix(dir: &Path) -> Result<MatrixResult, String> {
    let all = load_all(dir)?;
    let mut cells = Vec::new();
    let mut total_runs = 0;
    let mut cross_matches = 0;

    for runner in &all {
        for donor in &all {
            let mut executed = 0;
            let mut matched = 0;

            for test in &donor.tests {
                let result = runner.run(test.input.clone());
                total_runs += 1;
                if result.success {
                    executed += 1;
                }

                // Check if output matches donor's expected output
                let matches = test
                    .expect
                    .iter()
                    .all(|(k, v)| result.output.get(k) == Some(v));
                if matches {
                    matched += 1;
                }
                if matches && runner.name != donor.name {
                    cross_matches += 1;
                }
            }

            cells.push(MatrixCell {
                runner: runner.name.clone(),
                test_from: donor.name.clone(),
                total: donor.tests.len(),
                executed,
                matched,
            });
        }
    }

    Ok(MatrixResult {
        cells,
        total_runs,
        cross_matches,
    })
}
