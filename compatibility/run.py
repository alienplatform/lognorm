#!/usr/bin/env python3
"""Run pinned real loggers; validate fresh output, optionally refresh raw fixtures."""

import argparse
from decimal import Decimal
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
HERE = ROOT / "compatibility"
PAYLOAD = {
    "nested": {"path": "/example", "statusCode": 401},
    "items": [True, None, 7, "λ"],
    "enabled": True,
}


def run(command, cwd=ROOT, input=None):
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            input=input,
            text=True,
            capture_output=True,
            check=True,
            timeout=300,
        )
    except subprocess.CalledProcessError as error:
        sys.stderr.write(error.stdout or "")
        sys.stderr.write(error.stderr or "")
        raise
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
            "pino-multistream",
            "pino-mixin",
            "pino-nested-key",
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
            "loguru-time-exponent",
            "loguru-error",
            "loguru-text",
        ],
        "rust": [
            "tracing-nested",
            "tracing-flat",
            "tracing-span",
            "tracing-no-time",
            "tracing-no-target",
            "tracing-flat-span",
            "tracing-text",
            "env-logger-text",
        ],
    }
    groups["go"] = ["hclog-json", "hclog-child", "hclog-error"]
    for runtime, names in groups.items():
        for name in names:
            text = name.endswith("-text")
            payload_path = ["payload"]
            if name == "pino-nested-key":
                payload_path = ["data", "payload"]
            if name == "winston-metadata":
                payload_path = ["metadata", "payload"]
            if name.startswith("loguru-"):
                payload_path = ["record", "extra", "payload"]
            if name.startswith("tracing-") and name not in [
                "tracing-flat",
                "tracing-flat-span",
            ]:
                payload_path = ["fields", "payload"]
            has_time = name.startswith(
                ("pino-", "bunyan-", "loguru-", "tracing-", "hclog-")
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
    if name == "loguru-time-exponent":
        assert normalized["timestamp"] == {"seconds": 0, "nanos": 1000}, name
        assert "e-06" in raw or "e-6" in raw, (name, "producer must emit an exponent")
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
        if name == "pino-mixin":
            assert source["component"] == "synthetic", name
        if name == "hclog-error":
            assert source["error"] == "synthetic failure", name
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
            scalar_fields = (
                source
                if name in ["tracing-flat", "tracing-flat-span"]
                else source["fields"]
            )
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
    parser.add_argument("--runtime", choices=["node", "python", "rust", "go"])
    parser.add_argument(
        "--producer-root",
        type=Path,
        help="isolated producer directories for version sweeps",
    )
    args = parser.parse_args()
    if args.refresh and args.runtime:
        parser.error("fixture refresh requires every runtime")
    parser_root = args.producer_root or HERE
    selected = {args.runtime} if args.runtime else {"node", "python", "rust", "go"}
    if args.refresh and args.producer_root:
        parser.error("fixture refresh requires the checked-in pinned producers")
    if "node" in selected:
        run(["npm", "ci", "--ignore-scripts"], parser_root / "node")
    if "python" in selected:
        run(["uv", "sync", "--locked", "--project", str(parser_root / "python")])
    if "rust" in selected:
        run(
            [
                "cargo",
                "build",
                "--locked",
                "--manifest-path",
                str(parser_root / "rust/Cargo.toml"),
            ]
        )
    if "go" in selected:
        run(["go", "build", "-mod=readonly", "-o", "producer", "."], parser_root / "go")
    run(["cargo", "build", "--locked", "--example", "normalize"])
    cases = [case for case in scenarios() if case["runtime"] in selected]
    for case in cases:
        runtime, name = case["runtime"], case["scenario"]
        if runtime == "node":
            command = ["node", str(parser_root / "node/producer.mjs"), name]
        elif runtime == "python":
            command = [
                str(parser_root / "python/.venv/bin/python"),
                str(parser_root / "python/producer.py"),
                name,
            ]
        elif runtime == "rust":
            command = [
                str(parser_root / "rust/target/debug/lognorm-compatibility-producers"),
                name,
            ]
        else:
            command = [str(parser_root / "go/producer"), name]
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
