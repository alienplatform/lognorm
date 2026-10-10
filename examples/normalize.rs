//! Normalize newline-delimited records for compatibility experiments.
use std::io::{self, BufRead, Write};

use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        let line = line?;
        let log = lognorm::parse(&line);
        let timestamp = log.timestamp().map(|time| {
            json!({
                "seconds": time.unix_seconds(), "nanos": time.nanoseconds()
            })
        });
        serde_json::to_writer(
            &mut output,
            &json!({
                "original": log.original(),
                "message": log.message(),
                "fields": log.fields(),
                "severity": log.severity().map(|level| level.as_str()),
                "timestamp": timestamp,
                "outcome": format!("{:?}", log.outcome()),
            }),
        )?;
        writeln!(output)?;
    }
    Ok(())
}
