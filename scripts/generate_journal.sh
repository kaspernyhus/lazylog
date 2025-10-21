#!/bin/bash
# Generate random log messages to journalctl until terminated

# Array of log levels
LOG_LEVELS=("INFO" "WARNING" "ERROR" "DEBUG")

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

send_log_line() {
    level=$1
    component=$2
    message=$3
    request_id=$4
    duration=$5

    # Map log level to syslog priority
    case "$level" in
        "INFO") priority="info" ;;
        "WARNING") priority="warning" ;;
        "ERROR") priority="err" ;;
        "DEBUG") priority="debug" ;;
        *) priority="info" ;;
    esac

    # Send to journalctl with identifier
    echo "$level $component: $message (request_id=$request_id, duration=${duration}ms)" | systemd-cat -t lazylog-test -p $priority
}

while true; do
    # Randomly trigger a burst (10% chance)
    if [ $((RANDOM % 10)) -eq 0 ]; then
        echo "BURST: Sending 100 log lines rapidly..." >&2
        for i in {1..100}; do
            level=${LOG_LEVELS[$RANDOM % ${#LOG_LEVELS[@]}]}
            component=${COMPONENTS[$RANDOM % ${#COMPONENTS[@]}]}

            if [ "$level" = "ERROR" ]; then
                message=${ERRORS[$RANDOM % ${#ERRORS[@]}]}
            else
                message=${MESSAGES[$RANDOM % ${#MESSAGES[@]}]}
            fi

            request_id=$(printf "%08x" $RANDOM)
            duration=$((RANDOM % 1000))

            send_log_line "$level" "$component" "$message" "$request_id" "$duration"
        done
        echo "BURST: Complete" >&2
    else
        # Normal single log line
        level=${LOG_LEVELS[$RANDOM % ${#LOG_LEVELS[@]}]}
        component=${COMPONENTS[$RANDOM % ${#COMPONENTS[@]}]}

        if [ "$level" = "ERROR" ]; then
            message=${ERRORS[$RANDOM % ${#ERRORS[@]}]}
        else
            message=${MESSAGES[$RANDOM % ${#MESSAGES[@]}]}
        fi

        request_id=$(printf "%08x" $RANDOM)
        duration=$((RANDOM % 1000))

        send_log_line "$level" "$component" "$message" "$request_id" "$duration"
    fi

    # Random delay between 0.1 and 1.0 seconds
    sleep 0.$(printf "%01d" $((RANDOM % 10)))
done
