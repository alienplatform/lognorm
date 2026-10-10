use serde_json::json;
use tracing::{info_span, warn};

fn emit(scenario: &str) {
    let payload = json!({"nested": {"path": "/example", "statusCode": 401}, "items": [true, null, 7, "λ"], "enabled": true});
    let span = info_span!("operation", component = "synthetic");
    let _entered = (scenario == "tracing-span").then(|| span.enter());
    warn!(scenario, payload = ?payload, status_code = 401, enabled = true, "example {scenario}");
}

fn main() {
    let scenario = std::env::args().nth(1).expect("scenario argument");
    let base = tracing_subscriber::fmt().with_writer(std::io::stdout).with_ansi(false);
    if scenario == "tracing-text" {
        base.without_time().init();
    } else if scenario == "tracing-flat" {
        base.json().flatten_event(true).init();
    } else if scenario == "tracing-no-time" {
        base.json().without_time().init();
    } else {
        base.json().init();
    }
    emit(&scenario);
}
