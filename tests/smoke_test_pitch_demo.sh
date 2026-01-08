#!/usr/bin/env bash
set -e

# Mock cargo to simulate build/run steps
function cargo() {
    echo "[mock] cargo $@"
    
    local bin_name=""
    local output_file=""
    local emit_json=""
    
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --bin)
                bin_name="$2"
                shift 2
                ;;
            --emit-json)
                emit_json="$2"
                shift 2
                ;;
            *)
                if [[ "$bin_name" == "stimulus" && -z "$output_file" && "$1" != -* ]]; then
                    output_file="$1"
                fi
                if [[ "$bin_name" == "morphogenetic-security" && "$1" == "--telemetry" ]]; then
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
    if [[ -n "$emit_json" ]]; then
        echo "[mock] creating $emit_json"
        touch "$emit_json"
    fi
}

# Mock python3
function python3() {
    echo "[mock] python3 $@"
    # If script is prepare_telemetry_dashboard.py, check for --output
    local script="$1"
    if [[ "$script" == *"prepare_telemetry_dashboard.py"* ]]; then
        local output_csv=""
        local output_vega=""
        while [[ $# -gt 0 ]]; do
            if [[ "$1" == "--output" ]]; then output_csv="$2"; fi
            if [[ "$1" == "--vega-lite" ]]; then output_vega="$2"; fi
            shift
        done
        if [[ -n "$output_csv" ]]; then touch "$output_csv"; fi
        if [[ -n "$output_vega" ]]; then touch "$output_vega"; fi
    fi
}

export -f cargo
export -f python3

echo "Running scripts/pitch_demo.sh with mocks..."
bash -c "source scripts/pitch_demo.sh"

echo "Smoke test complete."
