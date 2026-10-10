"""Seed a fresh corpus with actual producer records and independent edge cases."""

import json
from pathlib import Path
import sys

root = Path(__file__).resolve().parent.parent
output = Path(sys.argv[1])
output.mkdir(parents=True, exist_ok=True)
if any(output.iterdir()):
    raise ValueError(
        "Choose an empty corpus directory; existing fuzz discoveries are never deleted"
    )

cases = [
    case["input"]
    for case in json.loads((root / "compatibility/fixtures.json").read_text())
]
cases.extend(
    [
        '{"x":1,"\\u0078":2}',
        '{"$serde_json::private::Number":"123"}',
        '{"n":0.123456789012345678901}',
        '{"n":1e400}',
        '{"msg":"\\uD800"}',
        '{"msg":"ready"} {}',
        "\x1b(Bready",
        "\x1bP1|hidden λ\x1b\\ready",
        "\x1bλready",
        "\x1b[31λready",
        '{"nested":' + "[" * 126 + "0" + "]" * 126 + "}",
        '{"n":1e999999999999999999999}',
        '{"values":[' + "0," * 1000 + "0]}",
        '{"nested":' + "[" * 128 + "0" + "]" * 128 + "}",
    ]
)
for index, record in enumerate(cases):
    (output / str(index)).write_text(record)
print(f"Seeded {len(cases)} records in {output}")
