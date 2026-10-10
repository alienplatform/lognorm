#!/usr/bin/env python3
"""Verify the boundary between native console output and complete log records."""

from decimal import Decimal
import json
import subprocess

from run import ROOT, run, same_json


def main():
    # Native console inspection is text and can span physical lines. A JSON
    # console call is a separate, structured record. Both use fresh generic data.
    script = """
    const payload = {
      items: Array.from({length: 12}, (_, i) => ({index: i, enabled: true})),
      nested: {count: 3, label: 'example'}
    };
    console.warn('example console output', payload);
    console.log(JSON.stringify({message: 'example JSON output', level: 'WARN', payload}));
    """
    emitted = subprocess.run(
        ["node", "--input-type=module", "-e", script],
        capture_output=True,
        text=True,
        check=True,
    )
    inspected = emitted.stderr.removesuffix("\n").split("\n")
    structured = emitted.stdout.removesuffix("\n").split("\n")
    assert len(inspected) > 1, (
        "native console object must exercise physical-line splitting"
    )
    assert len(structured) == 1, "JSON console output must remain one record"
    records = inspected + structured
    run(["cargo", "build", "--locked", "--example", "normalize"])
    output = run(
        [str(ROOT / "target/debug/examples/normalize")],
        input="\n".join(records) + "\n",
    )
    normalized = [
        json.loads(line, parse_float=Decimal, parse_int=Decimal)
        for line in output.splitlines()
    ]
    assert len(normalized) == len(records)
    for raw, log in zip(inspected, normalized[:-1]):
        assert log["original"] == raw and log["message"] == raw
        assert log["fields"] == {} and log["severity"] is None
        assert log["timestamp"] is None
    source = json.loads(structured[0], parse_float=Decimal, parse_int=Decimal)
    log = normalized[-1]
    assert log["original"] == structured[0]
    assert log["message"] == "example JSON output" and log["severity"] == "WARN"
    assert same_json(log["fields"], source)
    print(
        f"Verified {len(inspected)} native console lines and one structured console record"
    )


if __name__ == "__main__":
    main()
