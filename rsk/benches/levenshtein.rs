use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rsk::{levenshtein, fuzzy_search};

fn bench_levenshtein(c: &mut Criterion) {
    let mut group = c.benchmark_group("levenshtein");

    group.bench_function("short_strings", |b| {
        b.iter(|| levenshtein(black_box("kitten"), black_box("sitting")))
    });

    let medium1 = "The quick brown fox jumps over the lazy dog";
    let medium2 = "A quick brown dog jumps over the lazy fox";
    group.bench_function("medium_strings", |b| {
        b.iter(|| levenshtein(black_box(medium1), black_box(medium2)))
    });

    let long1 = "a".repeat(200) + &"b".repeat(200);
    let long2 = "b".repeat(200) + &"a".repeat(200);
    group.bench_function("long_strings", |b| {
        b.iter(|| levenshtein(black_box(&long1), black_box(&long2)))
    });

    group.finish();
}

fn bench_fuzzy_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("fuzzy_search");

    let candidates: Vec<String> = (0..100).map(|i| format!("skill-{}", i)).collect();
    group.bench_function("100_candidates", |b| {
        b.iter(|| fuzzy_search(black_box("test-skill"), black_box(&candidates), black_box(10)))
    });

    let candidates_large: Vec<String> = (0..1000).map(|i| format!("skill-{}", i)).collect();
    group.bench_function("1000_candidates", |b| {
        b.iter(|| fuzzy_search(black_box("test-skill"), black_box(&candidates_large), black_box(10)))
    });

    group.finish();
}

criterion_group!(benches, bench_levenshtein, bench_fuzzy_search);
criterion_main!(benches);
