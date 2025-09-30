#!/bin/bash
# Generate random log messages to journalctl until terminated

# Array of log levels
LOG_LEVELS=("info" "warning" "err" "debug")

# Array of sample components/services
COMPONENTS=("web-server" "database" "cache" "auth-service" "api-gateway" "worker")

# Array of sample messages
MESSAGES=(
    "Processing request"
    "Connection established"
    "Cache miss"
    "Query executed"
    "Authentication successful"
    "Request timeout"
    "Rate limit exceeded"
    "Healthcheck passed"
    "Configuration reloaded"
    "Job queued"
    "Transaction committed"
    "Session created"
    "File uploaded"
    "Backup completed"
    "Metrics collected"
)

# Array of sample errors
ERRORS=(
    "Connection refused"
    "Timeout waiting for response"
    "Invalid credentials"
    "Resource not found"
    "Permission denied"
    "Out of memory"
    "Deadlock detected"
    "Disk space low"
)

echo "Starting random log generation to journalctl..." >&2
echo "Press Ctrl+C to stop" >&2
echo "" >&2

# Trap SIGINT and SIGTERM for clean exit
trap 'echo "Stopping log generation..." >&2; exit 0' SIGINT SIGTERM

while true; do
    # Pick random values
    level=${LOG_LEVELS[$RANDOM % ${#LOG_LEVELS[@]}]}
    component=${COMPONENTS[$RANDOM % ${#COMPONENTS[@]}]}

    # Pick message based on level
    if [ "$level" = "err" ]; then
        message=${ERRORS[$RANDOM % ${#ERRORS[@]}]}
    else
        message=${MESSAGES[$RANDOM % ${#MESSAGES[@]}]}
    fi

    # Generate random request ID
    request_id=$(printf "%08x" $RANDOM)

    # Generate random duration
    duration=$((RANDOM % 1000))

    # Send to journalctl with identifier
    echo "[$level] $component: $message (request_id=$request_id, duration=${duration}ms)" | systemd-cat -t lazylog-test -p $level

    # Random delay between 0.1 and 1.0 seconds
    sleep 0.$(printf "%01d" $((RANDOM % 10)))
done
