# Recognition and preservation

Field names alone can be misleading. A business object may contain `level: 30`,
and multiple timestamp or severity fields may disagree. Automatic recognition
must preserve data without silently assigning the wrong meaning.

## Preservation

`fields()` returns the complete decoded root object, including fields used by
metadata accessors. Preserve booleans, strings, nulls, arrays, nested objects,
and numbers. Large integers and decimal representations must not silently lose
precision. Duplicate keys anywhere in an object cause raw fallback because a
map cannot represent both values faithfully. Objects containing the decoder's
reserved `$serde_json::private::Number` key also fall back intact; interpreting
that key as ordinary data through the arbitrary-precision visitor would be unsafe.

`original()` returns the exact input, including whitespace and control
characters. Display cleanup may remove ANSI escapes from `message()` but must
never alter `original()` or values in `fields()`. Oversized input bypasses
display cleanup as well as decoding so the size limit bounds work.

## Messages

Recognize string `msg` and `message` fields. Recognize `fields.message` in a
tracing envelope. Recognize structlog's string `event` when logger metadata
supports that interpretation. Recognize `record.message` in serialized Loguru.

When multiple message candidates disagree, use the original line. If there is
no message, use the original line. Never stringify a nested business object and
call it the message merely because its key is `body` or `log`.

Nested tracing fields remain nested in the preserved map. A flattened tracing
configuration is also supported. Empty string messages are valid.

## Severity

Recognize common textual levels, case-insensitively, including `warning`,
`critical`, and Python's level names. Recognize explicit OpenTelemetry numeric
severity fields on their 1–24 scale. Recognize Python's `levelno` scale. Recognize
Pino and Bunyan numeric `level` only when the rest of the record supports their
conventions, such as `msg` and `time`, or `msg`, `pid`, and `hostname`
when timestamps are disabled. Loguru exposes textual and numeric level
information under `record.level`.

Two recognized severities that disagree produce `None`. Unrecognized custom
levels remain in the fields. Text content and HTTP status codes do not determine
severity. Collectors decide their own stream fallback when severity is absent.

## Time

Recognize RFC 3339 timestamps under common logger timestamp keys. Recognize
Pino/Bunyan `time` milliseconds and Loguru's explicit seconds timestamp.
Timestamp keys that explicitly name a unit may use that unit. Do not infer
seconds versus milliseconds solely from the number of digits in an arbitrary
business field.

Keep seconds signed and nanoseconds in `0..1_000_000_000`. Preserve subsecond
precision and reject out-of-range values. Explicit numeric seconds support
exponent notation using checked decimal arithmetic; nonzero sub-nanosecond
values are left unresolved rather than rounded. Conflicting recognized timestamps
produce `None`. No wall clock is consulted by the parser.

## Text and failures

Ordinary text is a successful passthrough. JSON arrays and scalar values also
pass through. A leading Unicode BOM is passed through as text rather than
stripped. Invalid JSON and resource limits have distinct outcomes. None of
these cases may panic or drop the original input.

Pretty text can be decolorized for display, but structured fields that were
never emitted cannot be recovered. Multiline text must be assembled before
calling the parser.

Display cleanup uses a maintained ANSI state machine. Non-ASCII text after a
malformed ESC/CSI prefix remains visible; OSC/DCS payloads are discarded. Tabs
and newlines outside control strings remain. Cleanup does not emulate a screen.
