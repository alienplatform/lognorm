import pino from "pino";
import winston from "winston";
import bunyan from "bunyan";

const scenario = process.argv[2];
const message = `example ${scenario}`;
const payload = {
  nested: { path: "/example", statusCode: 401 },
  items: [true, null, 7, "λ"],
  enabled: true,
};
const context = { scenario, payload };
const error = new Error("synthetic failure");

if (scenario.startsWith("pino-")) {
  const options = {
    base: { pid: 42, hostname: "example-host" },
    timestamp: () => ',"time":1700000000123',
  };
  if (scenario === "pino-uppercase")
    options.formatters = { level: (label) => ({ level: label.toUpperCase() }) };
  if (scenario === "pino-iso")
    options.timestamp = () => ',"time":"2023-11-14T22:13:20.123Z"';
  if (scenario === "pino-no-time") options.timestamp = false;
  if (scenario === "pino-message-key") options.messageKey = "message";
  if (scenario === "pino-base-null") options.base = null;
  if (scenario === "pino-custom-level") options.customLevels = { audit: 35 };
  let logger = pino(options, pino.destination({ sync: true }));
  if (scenario === "pino-child")
    logger = logger.child({ component: "synthetic" });
  if (scenario === "pino-custom-level") logger.audit(context, message);
  else
    logger.warn(
      scenario === "pino-error" ? { ...context, err: error } : context,
      message,
    );
} else if (scenario.startsWith("winston-")) {
  const formats = [];
  if (scenario === "winston-error")
    formats.push(winston.format.errors({ stack: true }));
  if (scenario === "winston-time")
    formats.push(
      winston.format.timestamp({ format: () => "2023-11-14T22:13:20.123Z" }),
    );
  if (scenario === "winston-metadata") formats.push(winston.format.metadata());
  formats.push(
    scenario === "winston-text"
      ? winston.format.printf((info) => info.message)
      : winston.format.json(),
  );
  const logger = winston.createLogger({
    format: winston.format.combine(...formats),
    transports: [new winston.transports.Console()],
  });
  if (scenario === "winston-error") {
    error.message = message;
    Object.assign(error, context);
    logger.warn(error);
  } else logger.warn(message, context);
  logger.end();
} else if (scenario.startsWith("bunyan-")) {
  let logger = bunyan.createLogger({
    name: "synthetic",
    hostname: "example-host",
    pid: 42,
    serializers: bunyan.stdSerializers,
    stream: process.stdout,
  });
  if (scenario === "bunyan-child")
    logger = logger.child({ component: "synthetic" });
  logger.warn(
    scenario === "bunyan-error" ? { ...context, err: error } : context,
    message,
  );
} else if (scenario === "console-text") {
  console.log(message);
} else {
  throw new Error(`Unknown scenario: ${scenario}`);
}
