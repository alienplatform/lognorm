//! Real producer fixtures through normalization, HTTP transport, and OTLP decode.
#![cfg(feature = "otlp")]

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

use opentelemetry_proto::tonic::{
    collector::logs::v1::{ExportLogsServiceRequest, ExportLogsServiceResponse},
    common::v1::{any_value::Value as OtlpValue, AnyValue},
    logs::v1::{ResourceLogs, ScopeLogs},
};
use prost::Message;
use serde_json::Value;

fn decode_value(value: &AnyValue) -> Value {
    match &value.value {
        None => Value::Null,
        Some(OtlpValue::StringValue(value)) => Value::String(value.clone()),
        Some(OtlpValue::BoolValue(value)) => Value::Bool(*value),
        Some(OtlpValue::IntValue(value)) => Value::Number((*value).into()),
        Some(OtlpValue::DoubleValue(value)) => serde_json::json!(value),
        Some(OtlpValue::ArrayValue(value)) => {
            Value::Array(value.values.iter().map(decode_value).collect())
        }
        Some(OtlpValue::KvlistValue(value)) => Value::Object(
            value
                .values
                .iter()
                .map(|entry| {
                    (
                        entry.key.clone(),
                        decode_value(entry.value.as_ref().unwrap()),
                    )
                })
                .collect(),
        ),
        Some(OtlpValue::BytesValue(_)) => panic!("unexpected bytes"),
    }
}

#[test]
fn producer_records_survive_otlp_http_export() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../compatibility/fixtures.json")).unwrap();
    let logs = cases
        .iter()
        .map(|case| {
            let parsed = lognorm::parse(case["input"].as_str().unwrap());
            let mut record = lognorm::otlp::log_record(&parsed);
            record.observed_time_unix_nano = 1_800_000_000_000_000_000;
            record
        })
        .collect();
    let request = ExportLogsServiceRequest {
        resource_logs: vec![ResourceLogs {
            scope_logs: vec![ScopeLogs {
                log_records: logs,
                ..Default::default()
            }],
            ..Default::default()
        }],
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let receiver = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut reader = BufReader::new(&mut socket);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert_eq!(line, "POST /v1/logs HTTP/1.1\r\n");
        let mut length = None;
        let mut protobuf = false;
        loop {
            line.clear();
            assert!(reader.read_line(&mut line).unwrap() > 0);
            if line == "\r\n" {
                break;
            }
            if let Some(value) = line.strip_prefix("Content-Length: ") {
                length = Some(value.trim().parse::<usize>().unwrap());
            }
            protobuf |= line == "Content-Type: application/x-protobuf\r\n";
        }
        assert!(protobuf);
        let mut body = vec![0; length.unwrap()];
        reader.read_exact(&mut body).unwrap();
        let decoded = ExportLogsServiceRequest::decode(body.as_slice()).unwrap();
        let response = ExportLogsServiceResponse::default().encode_to_vec();
        write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", response.len()).unwrap();
        socket.write_all(&response).unwrap();
        decoded
    });
    let payload = request.encode_to_vec();
    let mut sender = TcpStream::connect(address).unwrap();
    sender
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    write!(sender, "POST /v1/logs HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", payload.len()).unwrap();
    sender.write_all(&payload).unwrap();
    let mut response = Vec::new();
    sender.read_to_end(&mut response).unwrap();
    assert!(response.starts_with(b"HTTP/1.1 200 OK\r\n"));
    let decoded = receiver.join().unwrap();
    let records = &decoded.resource_logs[0].scope_logs[0].log_records;
    assert_eq!(records.len(), cases.len());
    for (case, record) in cases.iter().zip(records) {
        let expected = &case["expected"];
        let name = case["scenario"].as_str().unwrap();
        let message = if expected["structured"] == true {
            &expected["message"]
        } else {
            &case["input"]
        };
        assert_eq!(
            decode_value(record.body.as_ref().unwrap()),
            *message,
            "{name}"
        );
        assert_eq!(
            record.severity_number,
            if expected["severity"].is_null() {
                0
            } else {
                13
            },
            "{name}"
        );
        assert_eq!(
            record.time_unix_nano != 0,
            expected["timestamp"] == true,
            "{name}"
        );
        assert_eq!(record.observed_time_unix_nano, 1_800_000_000_000_000_000);
        if expected["structured"] == true {
            assert_eq!(record.attributes.len(), 1, "{name}");
            assert_eq!(record.attributes[0].key, "app");
            assert_eq!(
                decode_value(record.attributes[0].value.as_ref().unwrap()),
                serde_json::from_str::<Value>(case["input"].as_str().unwrap()).unwrap(),
                "{name}"
            );
        } else {
            assert!(record.attributes.is_empty());
        }
    }
}

#[test]
fn unsupported_otlp_numbers_use_exact_strings_and_negative_time_is_unspecified() {
    let parsed = lognorm::parse(
        r#"{"big":18446744073709551615,"precise":0.123456789012345678901,"ordinary":1.5,"nothing":null,"time_ms":-1}"#,
    );
    let record = lognorm::otlp::log_record(&parsed);
    let app = decode_value(record.attributes[0].value.as_ref().unwrap());
    assert_eq!(app["big"], "18446744073709551615");
    assert_eq!(app["precise"], "0.123456789012345678901");
    assert_eq!(app["ordinary"], 1.5);
    assert!(app["nothing"].is_null());
    assert_eq!(record.time_unix_nano, 0);
}
