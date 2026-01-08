#!/usr/bin/env bash
set -e

# Mock cargo to simulate build/run steps
function cargo() {
    echo "[mock] cargo $@"
    
    # Simple arg parsing to find what we need to simulate
    local bin_name=""
    local output_file=""
    
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --bin)
                bin_name="$2"
                shift 2
                ;;
            *)
                # Check for positional args or flags for outputs
                if [[ "$bin_name" == "stimulus" && -z "$output_file" && "$1" != -* ]]; then
                    output_file="$1"
                fi
                if [[ "$bin_name" == "morphogenetic-security" && "$1" == "--telemetry" ]]; then
                    output_file="$2"
                fi
                if [[ "$bin_name" == "adversarial_cycle" && "$1" == "--emit-json" ]]; then
                    output_file="$2"
                fi
                shift
                ;;
        esac
    done
    
    if [[ -n "$output_file" ]]; then
        echo "[mock] creating $output_file"
        touch "$output_file"
    fi
}

# Mock python3 to simulate analysis steps
function python3() {
    echo "[mock] python3 $@"
    local script="$1"
    
    if [[ "$script" == *"prepare_telemetry_dashboard.py"* ]]; then
        # Find --output flag
        local output_file=""
        while [[ $# -gt 0 ]]; do
            if [[ "$1" == "--output" ]]; then
                output_file="$2"
            fi
            shift
        done
        
        if [[ -n "$output_file" ]]; then
            echo "[mock] creating dashboard $output_file"
            touch "$output_file"
        fi
    fi
}

export -f cargo
export -f python3

# Run the actual script
# We need to run it in a way that it uses our mocked functions.
# Sourcing it runs it in current shell.
# Executing it starts new shell (which might not inherit functions unless we use specific invocation).
# Bash allows exporting functions.

echo "Running scripts/run_attack_simulation.sh with mocks..."
bash -c "source scripts/run_attack_simulation.sh"

echo "Smoke test complete."
