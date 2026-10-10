//! Throughput of normalization across representative record sizes.
// Criterion generates a public runner without rustdoc.
#![allow(missing_docs)]

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn cases() -> Vec<(String, String)> {
    let mut cases: Vec<_> = [
        ("text", "Server ready".to_owned()),
        ("invalid", "{broken".to_owned()),
        ("small", r#"{"level":30,"time":1700000000123,"msg":"ready"}"#.to_owned()),
        ("nested", r#"{"level":"WARN","msg":"Request rejected","statusCode":401,"req":{"path":"/v1/check","tags":["a","b"]}}"#.to_owned()),
        ("exception", serde_json::json!({"level":"ERROR","message":"failed","error":{"stack":"frame\n".repeat(500)}}).to_string()),
        ("limit", "x".repeat(1024 * 1024 + 1)),
        ("ansi", "\x1b[31mRequest rejected λ\x1b[0m".to_owned()),
    ].into_iter().map(|(name, line)| (name.to_owned(), line)).collect();
    let fixtures: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../compatibility/fixtures.json")).unwrap();
    for name in [
        "pino-default",
        "tracing-nested",
        "loguru-json",
        "loguru-time-exponent",
        "structlog-json",
    ] {
        let fixture = fixtures
            .iter()
            .find(|case| case["scenario"] == name)
            .unwrap();
        cases.push((
            name.to_owned(),
            fixture["input"].as_str().unwrap().to_owned(),
        ));
    }
    cases
}

fn normalize(c: &mut Criterion) {
    let mut group = c.benchmark_group("normalize");
    for (name, line) in cases() {
        if name == "limit" {
            // Rejected by length without scanning; byte throughput is misleading.
            group.throughput(Throughput::Elements(1));
        } else {
            group.throughput(Throughput::Bytes(line.len() as u64));
        }
        group.bench_with_input(BenchmarkId::from_parameter(name), &line, |b, line| {
            b.iter(|| lognorm::parse(black_box(line)));
        });
    }
    group.finish();
}

#[cfg(feature = "otlp")]
fn otlp(c: &mut Criterion) {
    let mut group = c.benchmark_group("normalize_and_otlp");
    for (name, line) in cases().into_iter().filter(|(name, _)| name != "limit") {
        group.throughput(Throughput::Bytes(line.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), &line, |b, line| {
            b.iter(|| {
                let parsed = lognorm::parse(black_box(line));
                lognorm::otlp::log_record(&parsed)
            });
        });
    }
    group.finish();
}

#[cfg(not(feature = "otlp"))]
fn otlp(_: &mut Criterion) {}

criterion_group!(benches, normalize, otlp);
criterion_main!(benches);
