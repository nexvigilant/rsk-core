//! Benchmarks for the execution engine module.
//!
//! Run with: cargo bench
//!
//! Benchmark scenarios:
//! - Small plans (10 modules) - typical skill execution
//! - Medium plans (100 modules) - complex orchestration
//! - Large plans (1000 modules) - stress testing
//!
//! Performance targets:
//! - 10 modules: < 50us
//! - 100 modules: < 500us
//! - 1000 modules: < 5ms

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use rsk::{build_execution_plan, ExecutionModule, EffortSize};

/// Generate a linear chain of modules (worst case for topological sort)
fn generate_linear_chain(n: usize) -> Vec<ExecutionModule> {
    (0..n)
        .map(|i| {
            let deps = if i > 0 {
                vec![format!("M{}", i - 1)]
            } else {
                vec![]
            };
            ExecutionModule::new(&format!("M{}", i), &format!("Module {}", i), deps)
                .with_effort(EffortSize::M)
                .with_risk(0.3)
        })
        .collect()
}

/// Generate a wide parallel DAG (best case for level parallelization)
fn generate_wide_parallel(n: usize) -> Vec<ExecutionModule> {
    let mut modules = vec![ExecutionModule::new("root", "Root", vec![])];

    for i in 0..n - 1 {
        modules.push(
            ExecutionModule::new(
                &format!("M{}", i),
                &format!("Parallel {}", i),
                vec!["root".to_string()],
            )
            .with_effort(EffortSize::S)
            .with_risk(0.2),
        );
    }

    modules
}

/// Generate a diamond DAG pattern (common in real workflows)
fn generate_diamond_dag(depth: usize, width: usize) -> Vec<ExecutionModule> {
    let mut modules = vec![];
    let mut prev_level: Vec<String> = vec![];

    for level in 0..depth {
        let mut current_level: Vec<String> = vec![];

        for i in 0..width {
            let id = format!("L{}M{}", level, i);
            let deps = if level == 0 {
                vec![]
            } else {
                prev_level.clone()
            };

            modules.push(
                ExecutionModule::new(&id, &format!("Level {} Module {}", level, i), deps)
                    .with_effort(EffortSize::M),
            );
            current_level.push(id);
        }

        prev_level = current_level;
    }

    modules
}

/// Benchmark plan building for different sizes
fn bench_build_plan(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_execution_plan");

    // Test different sizes
    for size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(
            BenchmarkId::new("linear_chain", size),
            size,
            |b, &size| {
                let modules = generate_linear_chain(size);
                b.iter(|| build_execution_plan(black_box(modules.clone())))
            },
        );

        group.bench_with_input(
            BenchmarkId::new("wide_parallel", size),
            size,
            |b, &size| {
                let modules = generate_wide_parallel(size);
                b.iter(|| build_execution_plan(black_box(modules.clone())))
            },
        );
    }

    group.finish();
}

/// Benchmark diamond DAG pattern (realistic workflow)
fn bench_diamond_dag(c: &mut Criterion) {
    let mut group = c.benchmark_group("diamond_dag");

    // depth x width combinations that sum to ~100 modules
    let configs = [(5, 20), (10, 10), (20, 5)];

    for (depth, width) in configs.iter() {
        let total = depth * width;
        group.throughput(Throughput::Elements(total as u64));

        group.bench_with_input(
            BenchmarkId::new("depth_x_width", format!("{}x{}", depth, width)),
            &(*depth, *width),
            |b, &(depth, width)| {
                let modules = generate_diamond_dag(depth, width);
                b.iter(|| build_execution_plan(black_box(modules.clone())))
            },
        );
    }

    group.finish();
}

/// Benchmark topological sort separately
fn bench_topological_sort(c: &mut Criterion) {
    use rsk::{SkillGraph, SkillNode};

    let mut group = c.benchmark_group("topological_sort");

    for size in [10, 100, 500].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::new("chain", size), size, |b, &size| {
            let mut graph = SkillGraph::new();
            for i in 0..size {
                let deps = if i > 0 {
                    vec![format!("N{}", i - 1)]
                } else {
                    vec![]
                };
                graph.add_node(SkillNode {
                    name: format!("N{}", i),
                    dependencies: deps,
                    adjacencies: vec![],
                });
            }
            b.iter(|| graph.topological_sort())
        });
    }

    group.finish();
}

/// Benchmark level parallelization
fn bench_level_parallelization(c: &mut Criterion) {
    use rsk::{SkillGraph, SkillNode};

    let mut group = c.benchmark_group("level_parallelization");

    for size in [10, 100, 500].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        // Wide parallel (best case - 2 levels)
        group.bench_with_input(BenchmarkId::new("wide", size), size, |b, &size| {
            let mut graph = SkillGraph::new();
            graph.add_node(SkillNode {
                name: "root".to_string(),
                dependencies: vec![],
                adjacencies: vec![],
            });
            for i in 0..size - 1 {
                graph.add_node(SkillNode {
                    name: format!("N{}", i),
                    dependencies: vec!["root".to_string()],
                    adjacencies: vec![],
                });
            }
            b.iter(|| graph.level_parallelization())
        });

        // Deep chain (worst case - n levels)
        group.bench_with_input(BenchmarkId::new("deep", size), size, |b, &size| {
            let mut graph = SkillGraph::new();
            for i in 0..size {
                let deps = if i > 0 {
                    vec![format!("N{}", i - 1)]
                } else {
                    vec![]
                };
                graph.add_node(SkillNode {
                    name: format!("N{}", i),
                    dependencies: deps,
                    adjacencies: vec![],
                });
            }
            b.iter(|| graph.level_parallelization())
        });
    }

    group.finish();
}

/// Benchmark critical path computation
fn bench_critical_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("critical_path");

    for size in [10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::new("diamond", size), size, |b, &size| {
            // Diamond: creates a DAG where critical path matters
            let modules = generate_diamond_dag(size / 5 + 1, 5);
            b.iter(|| build_execution_plan(black_box(modules.clone())))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_build_plan,
    bench_diamond_dag,
    bench_topological_sort,
    bench_level_parallelization,
    bench_critical_path,
);

criterion_main!(benches);
