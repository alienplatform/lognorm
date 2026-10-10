// Emit synthetic JSON records using the upstream hclog formatter.
package main

import (
	"errors"
	"os"
	"time"

	"github.com/hashicorp/go-hclog"
)

func main() {
	scenario := os.Args[1]
	logger := hclog.New(&hclog.LoggerOptions{
		Name: "synthetic", Output: os.Stdout, JSONFormat: true,
		TimeFn: func() time.Time { return time.Unix(1700000000, 123000000).UTC() },
	})
	payload := map[string]interface{}{
		"nested": map[string]interface{}{"path": "/example", "statusCode": 401},
		"items":  []interface{}{true, nil, 7, "λ"}, "enabled": true,
	}
	if scenario == "hclog-child" {
		logger = logger.With("component", "synthetic")
	}
	fields := []interface{}{"scenario", scenario, "payload", payload}
	if scenario == "hclog-error" {
		fields = append(fields, "error", errors.New("synthetic failure"))
	}
	logger.Warn("example "+scenario, fields...)
}
