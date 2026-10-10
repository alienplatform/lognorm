# Collector and dashboard integration

A readable log list is insufficient when application fields are still embedded
inside a string. Conversely, extracting fields is insufficient if the message
becomes unreadable or an exporter drops nested values.

## Collection boundary

Normalize complete application records after container framing and before
telemetry encoding. Worker output capture, host collectors, and Kubernetes
collectors use the same parser version and semantic contract.

Direct structured telemetry retains its own schema. Do not reinterpret its
string body as application JSON by default or normalize a record twice.

## Telemetry adapter

1. Use the selected message as the readable body.
2. Use recognized application severity and time when available.
3. Otherwise retain the collector's fallback severity and source time.
4. Export the original application object under an `app` attribute map.
5. Attach trusted infrastructure context separately.
6. Optionally retain original bytes using the target model's original-record
   field when exact recovery is required.

An application namespace avoids overwriting trusted attributes and preserves
literal dotted keys. A target that does not support nested maps needs an
explicit, tested encoding policy. The core never silently flattens values.

OpenTelemetry supports structured bodies and attribute collections. The default
adapter policy uses a string body and `app` map so list presentation and field
queries are both straightforward. Integer literals outside the target's signed 64-bit range use exact strings.
Other numbers use doubles only when their shortest JSON rendering has the same
decimal value as the source; exponent spelling alone does not change the type.
Values that cannot round-trip without loss use an explicit string representation; the adapter
documents that limitation and retains the original value where possible.

The optional `otlp` feature supplies a protobuf adapter. It represents JSON null
with an empty AnyValue. Unrepresentable timestamps use OTLP's unspecified zero;
callers supply their fallback time. Callers also set observed time and own
resource/scope context, batching, retries, and transport.

## Presentation and queries

The log list shows normalized body text and severity. The detail panel shows
expandable typed application fields alongside infrastructure context. Users can
filter a nested field such as `app.req.path` and a numeric field such as
`app.statusCode` without reparsing strings at query time.

Old stored records are not rewritten by changing the library. A display-only
compatibility path may improve old messages, but cannot be represented as an
ingestion or export fix.

## Acceptance

Run a real producer, collect its output, serialize and independently decode the
telemetry, ingest it, query nested and numeric fields, and inspect the log list
and detail panel. Verify that external exports retain the same fields. State
which boundaries were exercised; a local transport test does not prove a live
storage or dashboard integration.
