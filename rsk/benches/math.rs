use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rsk::{calculate_variance, is_prime, sha256_hash};

fn bench_math(c: &mut Criterion) {
    let mut group = c.benchmark_group("math");

    group.bench_function("variance", |b| {
        b.iter(|| calculate_variance(black_box(85.0), black_box(100.0)))
    });

    group.bench_function("is_prime_small", |b| {
        b.iter(|| is_prime(black_box(97)))
    });

    group.bench_function("is_prime_large", |b| {
        b.iter(|| is_prime(black_box(7919))) // 1000th prime
    });

    group.finish();
}

fn bench_crypto(c: &mut Criterion) {
    let mut group = c.benchmark_group("crypto");

    group.bench_function("sha256_short", |b| {
        b.iter(|| sha256_hash(black_box("hello world")))
    });

    let large_input = "x".repeat(10000);
    group.bench_function("sha256_10kb", |b| {
        b.iter(|| sha256_hash(black_box(&large_input)))
    });

    group.finish();
}

criterion_group!(benches, bench_math, bench_crypto);
criterion_main!(benches);
