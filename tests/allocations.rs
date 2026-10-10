//! Allocation evidence in a separate test executable to isolate instrumentation.
use std::{alloc::System, hint::black_box};

use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
fn report_allocations_and_require_zero_for_borrowed_fallbacks() {
    let oversized = "x".repeat(1024 * 1024 + 1);
    for (name, input) in [
        ("empty", ""),
        ("text", "Server ready"),
        ("limit", oversized.as_str()),
        ("small", r#"{"msg":"ready","level":"INFO","count":3}"#),
        ("cleanup", "\x1b[31mfailed\x1b[0m"),
    ] {
        let region = Region::new(GLOBAL);
        let log = black_box(lognorm::parse(black_box(input)));
        black_box(log.message());
        let stats = region.change();
        if ["empty", "text", "limit"].contains(&name) {
            assert_eq!(stats.allocations, 0, "{name}: {stats:?}");
            assert_eq!(stats.bytes_allocated, 0, "{name}: {stats:?}");
        }
        drop(log);
        println!(
            "{name}: allocations={}, allocated_bytes={}",
            stats.allocations, stats.bytes_allocated
        );
    }
}
