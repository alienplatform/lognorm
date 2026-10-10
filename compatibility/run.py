#!/usr/bin/env python3
"""Run pinned real loggers; validate fresh output, optionally refresh raw fixtures."""

import argparse
from decimal import Decimal
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
HERE = ROOT / "compatibility"
PAYLOAD = {
    "nested": {"path": "/example", "statusCode": 401},
    "items": [True, None, 7, "λ"],
    "enabled": True,
}


def run(command, cwd=ROOT, input=None):
    result = subprocess.run(
        command, cwd=cwd, input=input, text=True, capture_output=True, check=True
    )
    return result.stdout


def scenarios():
    records = []
    groups = {
        "node": [
            "pino-default",
            "pino-uppercase",
            "pino-iso",
            "pino-child",
            "pino-error",
            "pino-no-time",
            "pino-message-key",
            "pino-base-null",
            "pino-custom-level",
            "winston-json",
            "winston-time",
            "winston-metadata",
            "winston-error",
            "winston-text",
            "bunyan-default",
            "bunyan-child",
            "bunyan-error",
            "console-text",
        ],
        "python": [
            "python-json",
            "python-renamed",
            "python-error",
            "python-text",
            "structlog-json",
            "structlog-time",
            "structlog-error",
            "loguru-json",
            "loguru-error",
            "loguru-text",
        ],
        "rust": [
            "tracing-nested",
            "tracing-flat",
            "tracing-span",
            "tracing-no-time",
            "tracing-text",
            "env-logger-text",
        ],
    }
    for runtime, names in groups.items():
        for name in names:
            text = name.endswith("-text")
            payload_path = ["payload"]
            if name == "winston-metadata":
                payload_path = ["metadata", "payload"]
            if name.startswith("loguru-"):
                payload_path = ["record", "extra", "payload"]
            if name.startswith("tracing-") and name != "tracing-flat":
                payload_path = ["fields", "payload"]
            has_time = name.startswith(
                ("pino-", "bunyan-", "loguru-", "tracing-")
            ) or name in ["winston-time", "structlog-time"]
            records.append(
                {
                    "runtime": runtime,
                    "scenario": name,
                    "expected": {
                        "message": f"example {name}",
                        "severity": None
                        if text or name == "pino-custom-level"
                        else "WARN",
                        "structured": not text,
                        "timestamp": has_time
                        and not text
                        and not name.endswith("-no-time"),
                        "payloadPath": None if text else payload_path,
                        "payloadIsString": name.startswith("tracing-"),
                    },
                }
            )
    return records


def at(value, path):
    for key in path:
        value = value[key]
    return value


def same_json(left, right):
    # Python considers True == 1; preservation must also retain JSON value kinds.
    if type(left) is not type(right):
        return False
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(
            same_json(left[key], right[key]) for key in left
        )
    if isinstance(left, list):
        return len(left) == len(right) and all(
            same_json(a, b) for a, b in zip(left, right)
        )
    return left == right


def verify(case, normalized):
    name, expected, raw = case["scenario"], case["expected"], case["input"]
    assert normalized["original"] == raw, name
    if expected["structured"]:
        assert normalized["message"] == expected["message"], (name, normalized)
    else:
        assert normalized["message"] == raw and expected["message"] in raw, (
            name,
            normalized,
        )
    assert normalized["severity"] == expected["severity"], (name, normalized)
    assert (normalized["timestamp"] is not None) == expected["timestamp"], (
        name,
        normalized,
    )
    if expected["structured"]:
        source = json.loads(raw, parse_float=Decimal, parse_int=Decimal)
        assert same_json(normalized["fields"], source), (
            name,
            "typed field preservation",
        )
        payload = at(source, expected["payloadPath"])
        if expected["payloadIsString"]:
            # tracing Debug output is a string at the producer. Never invent a map.
            assert isinstance(payload, str) and "statusCode" in payload, (name, payload)
        else:
            assert payload == PAYLOAD, (name, payload)
        assert normalized["outcome"] == "Structured", name
        if name.endswith("-child"):
            assert source["component"] == "synthetic", name
        if name == "winston-error":
            assert (
                isinstance(source["stack"], str)
                and expected["message"] in source["stack"]
            ), name
        if name in ["pino-error", "bunyan-error"]:
            assert source["err"]["message"] == "synthetic failure" and isinstance(
                source["err"]["stack"], str
            ), name
        if name == "python-error":
            assert "ValueError: synthetic failure" in source["exc_info"], name
        if name == "structlog-error":
            assert "ValueError: synthetic failure" in source["exception"], name
        if name == "loguru-error":
            assert source["record"]["exception"]["value"] == "synthetic failure", name
        if name.startswith("tracing-"):
            scalar_fields = source if name == "tracing-flat" else source["fields"]
            assert (
                scalar_fields["status_code"] == 401 and scalar_fields["enabled"] is True
            ), name
        if name.endswith("-error"):
            assert "synthetic failure" in raw or name == "winston-error", (
                name,
                "missing real exception",
            )
        if name in ["pino-default", "pino-iso", "winston-time"]:
            assert normalized["timestamp"] == {
                "seconds": 1700000000,
                "nanos": 123000000,
            }, name
    else:
        assert normalized["fields"] == {}, name
        assert normalized["outcome"] == "Text", name


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--refresh",
        action="store_true",
        help="write verified raw producer output; expectations stay independent",
    )
    args = parser.parse_args()
    run(["npm", "ci", "--ignore-scripts"], HERE / "node")
    run(["uv", "sync", "--locked", "--project", str(HERE / "python")])
    run(
        ["cargo", "build", "--locked", "--manifest-path", str(HERE / "rust/Cargo.toml")]
    )
    run(["cargo", "build", "--locked", "--example", "normalize"])
    cases = scenarios()
    for case in cases:
        runtime, name = case["runtime"], case["scenario"]
        if runtime == "node":
            command = ["node", str(HERE / "node/producer.mjs"), name]
        elif runtime == "python":
            command = [
                str(HERE / "python/.venv/bin/python"),
                str(HERE / "python/producer.py"),
                name,
            ]
        else:
            command = [
                str(HERE / "rust/target/debug/lognorm-compatibility-producers"),
                name,
            ]
        output = run(command)
        assert output.endswith("\n"), (name, "unterminated producer output")
        lines = output[:-1].split("\n")
        assert len(lines) == 1 and lines[0], (
            name,
            "expected exactly one complete record",
            output,
        )
        case["input"] = lines[0]
    output = run(
        [str(ROOT / "target/debug/examples/normalize")],
        input="\n".join(case["input"] for case in cases) + "\n",
    )
    normalized = [
        json.loads(line, parse_float=Decimal, parse_int=Decimal)
        for line in output.splitlines()
    ]
    assert len(normalized) == len(cases), "missing normalized records"
    for case, result in zip(cases, normalized):
        verify(case, result)
        print(f"PASS {case['scenario']}")
    if args.refresh:
        # Only checked fixtures replace machine-specific source paths. Fresh
        # records above were verified exactly as emitted.
        for case in cases:
            case["input"] = (
                case["input"]
                .replace(str(ROOT), "/source/lognorm")
                .replace(str(Path.home()), "/home/example")
            )
        (HERE / "fixtures.json").write_text(
            json.dumps(cases, indent=2, ensure_ascii=False) + "\n"
        )
    print(f"Verified {len(cases)} real-library configurations")


if __name__ == "__main__":
    main()
