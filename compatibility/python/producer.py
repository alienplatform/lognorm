"""Emit one synthetic record using the actual configured logging library."""
import logging
import sys

import structlog
from loguru import logger as loguru
from pythonjsonlogger.json import JsonFormatter

scenario = sys.argv[1]
message = f"example {scenario}"
payload = {"nested": {"path": "/example", "statusCode": 401}, "items": [True, None, 7, "λ"], "enabled": True}
context = {"scenario": scenario, "payload": payload}

if scenario.startswith("python-"):
    logger = logging.getLogger("synthetic")
    logger.setLevel(logging.DEBUG)
    handler = logging.StreamHandler(sys.stdout)
    if scenario == "python-text":
        formatter = logging.Formatter("%(message)s")
    elif scenario == "python-renamed":
        formatter = JsonFormatter("%(levelname)s %(message)s", rename_fields={"levelname": "severity", "message": "msg"})
    else:
        formatter = JsonFormatter("%(levelname)s %(levelno)s %(message)s")
    handler.setFormatter(formatter)
    logger.addHandler(handler)
    if scenario == "python-error":
        try:
            raise ValueError("synthetic failure")
        except ValueError:
            logger.warning(message, extra=context, exc_info=True)
    else:
        logger.warning(message, extra=context)
elif scenario.startswith("structlog-"):
    processors = [structlog.processors.add_log_level]
    if scenario == "structlog-time":
        processors.append(structlog.processors.TimeStamper(fmt="iso", utc=True))
    if scenario == "structlog-error":
        processors.append(structlog.processors.format_exc_info)
    processors.append(structlog.processors.JSONRenderer())
    structlog.configure(processors=processors, logger_factory=structlog.PrintLoggerFactory(file=sys.stdout))
    logger = structlog.get_logger().bind(**context)
    if scenario == "structlog-error":
        try:
            raise ValueError("synthetic failure")
        except ValueError:
            logger.warning(message, exc_info=True)
    else:
        logger.warning(message)
elif scenario.startswith("loguru-"):
    loguru.remove()
    loguru.add(sys.stdout, serialize=scenario != "loguru-text", format="{message}", backtrace=False, diagnose=False)
    logger = loguru.bind(**context)
    if scenario == "loguru-error":
        try:
            raise ValueError("synthetic failure")
        except ValueError:
            logger.opt(exception=True).warning(message)
    else:
        logger.warning(message)
else:
    raise ValueError(f"Unknown scenario: {scenario}")
