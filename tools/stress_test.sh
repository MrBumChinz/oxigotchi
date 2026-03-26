#!/bin/bash
# Oxigotchi firmware stress test
# Tests rate/dwell/channel combinations to find real BCM43436B0 limits
# with full firmware patch stack (PSM, DPC, HardFault, frame padding)
#
# Safety: monitors for hard crashes, restores safe config on failure
# Each test runs for TEST_DURATION seconds

API="http://localhost:8080"
TEST_DURATION=120  # 2 minutes per combo
SAFE_RATE=1
SAFE_DWELL=5000
SAFE_CHANNELS='[1,6,11]'
LOG="/tmp/stress_test_$(date +%Y%m%d_%H%M%S).log"

log() { echo "[$(date '+%H:%M:%S')] $*" | tee -a "$LOG"; }

# Count PSM watchdog fires in dmesg from a given timestamp
count_psm_since() {
    local start_line=$1
    local n
    n=$(dmesg | tail -n +"$start_line" | grep -c "PSM's watchdog has fired" 2>/dev/null) || true
    echo "${n:-0}" | tr -d '[:space:]'
}

# Check if AO process is alive
ao_alive() {
    pgrep -f "angryoxide.*--interface" >/dev/null 2>&1
}

# Check for hard crashes (bus down, error -110, interface gone)
check_hard_crash() {
    local start_line=$1
    local n
    n=$(dmesg | tail -n +"$start_line" | grep -cE "bus is down|error.*-110|Set Channel failed.*-110|brcmf_sdio_bus_rxctl" 2>/dev/null) || true
    echo "${n:-0}" | tr -d '[:space:]'
}

# Check if wlan0mon still exists
wlan_alive() {
    ip link show wlan0mon >/dev/null 2>&1
}

# Restore safe config
restore_safe() {
    log "RESTORING safe config: rate=$SAFE_RATE dwell=$SAFE_DWELL channels=$SAFE_CHANNELS"
    curl -s -X POST "$API/api/rate" -H 'Content-Type: application/json' -d "{\"rate\": $SAFE_RATE}" >/dev/null 2>&1
    sleep 2
    curl -s -X POST "$API/api/channels" -H 'Content-Type: application/json' \
        -d "{\"channels\": $SAFE_CHANNELS, \"dwell_ms\": $SAFE_DWELL, \"autohunt\": false}" >/dev/null 2>&1
    sleep 5
}

# Run a single test combo
run_test() {
    local name="$1"
    local rate="$2"
    local dwell_ms="$3"
    local channels="$4"
    local autohunt="${5:-false}"

    log "==========================================="
    log "TEST: $name"
    log "  rate=$rate dwell=${dwell_ms}ms channels=$channels autohunt=$autohunt"
    log "==========================================="

    # Record dmesg line count before test
    local dmesg_lines
    dmesg_lines=$(dmesg | wc -l | tr -d '[:space:]')

    # Apply rate
    curl -s -X POST "$API/api/rate" -H 'Content-Type: application/json' \
        -d "{\"rate\": $rate}" >/dev/null 2>&1
    sleep 3  # wait for AO restart

    # Apply channel config
    curl -s -X POST "$API/api/channels" -H 'Content-Type: application/json' \
        -d "{\"channels\": $channels, \"dwell_ms\": $dwell_ms, \"autohunt\": $autohunt}" >/dev/null 2>&1
    sleep 3  # wait for AO restart

    # Verify AO is alive after config change
    if ! ao_alive; then
        log "RESULT: FAIL - AO died immediately after config change"
        restore_safe
        sleep 10
        return 1
    fi

    # Monitor for TEST_DURATION seconds
    local elapsed=0
    local check_interval=10
    local ao_died=false
    local hard_crash=false
    local wlan_gone=false

    while [ $elapsed -lt $TEST_DURATION ]; do
        sleep $check_interval
        elapsed=$((elapsed + check_interval))

        # Check AO process
        if ! ao_alive; then
            log "  [${elapsed}s] AO process died!"
            ao_died=true
            break
        fi

        # Check wlan0mon
        if ! wlan_alive; then
            log "  [${elapsed}s] wlan0mon disappeared!"
            wlan_gone=true
            break
        fi

        # Check hard crashes
        local hc
        hc=$(check_hard_crash "$dmesg_lines")
        if [ "${hc:-0}" -gt 0 ] 2>/dev/null; then
            log "  [${elapsed}s] Hard crash detected ($hc events)"
            hard_crash=true
            break
        fi

        # Progress (every 30s)
        if [ $((elapsed % 30)) -eq 0 ]; then
            local psm
            psm=$(count_psm_since "$dmesg_lines")
            log "  [${elapsed}s] OK - PSM fires: ${psm:-0}, AO: alive, wlan: up"
        fi
    done

    # Final stats
    local psm_total
    psm_total=$(count_psm_since "$dmesg_lines")
    local hc_total
    hc_total=$(check_hard_crash "$dmesg_lines")

    if $ao_died || $hard_crash || $wlan_gone; then
        log "RESULT: FAIL after ${elapsed}s"
        log "  PSM watchdog fires: $psm_total"
        log "  Hard crash events: $hc_total"
        log "  AO alive: $(ao_alive && echo yes || echo NO)"
        log "  wlan0mon: $(wlan_alive && echo up || echo DOWN)"
        restore_safe
        sleep 10
        return 1
    else
        log "RESULT: PASS (${TEST_DURATION}s)"
        log "  PSM watchdog fires: $psm_total"
        log "  Hard crash events: $hc_total"
    fi

    # Brief cooldown between tests
    sleep 5
    return 0
}

# ============================================================
log "Oxigotchi Firmware Stress Test"
log "Duration per test: ${TEST_DURATION}s"
log "Patches: PSM 0xFF, DPC, HardFault, frame padding 650B"
log ""

# First restore safe baseline
restore_safe
sleep 5

# Test matrix - progressive escalation
# Tier 1: Rate 1, push channels and dwell
run_test "R1-D2000-CH1611"     1 2000 '[1,6,11]'
run_test "R1-D2000-ALL13"      1 2000 '[1,2,3,4,5,6,7,8,9,10,11,12,13]'
run_test "R1-D1000-ALL13"      1 1000 '[1,2,3,4,5,6,7,8,9,10,11,12,13]'
run_test "R1-D500-ALL13"       1  500 '[1,2,3,4,5,6,7,8,9,10,11,12,13]'

# Tier 2: Rate 2, conservative dwell first
run_test "R2-D5000-CH1611"     2 5000 '[1,6,11]'
run_test "R2-D3000-CH1611"     2 3000 '[1,6,11]'
run_test "R2-D2000-CH1611"     2 2000 '[1,6,11]'
run_test "R2-D5000-ALL13"      2 5000 '[1,2,3,4,5,6,7,8,9,10,11,12,13]'
run_test "R2-D3000-ALL13"      2 3000 '[1,2,3,4,5,6,7,8,9,10,11,12,13]'

# Tier 3: Rate 3, very conservative
run_test "R3-D10000-CH1611"    3 10000 '[1,6,11]'
run_test "R3-D8000-CH1611"     3  8000 '[1,6,11]'
run_test "R3-D5000-CH1611"     3  5000 '[1,6,11]'
run_test "R3-D10000-ALL13"     3 10000 '[1,2,3,4,5,6,7,8,9,10,11,12,13]'

# Always restore safe at the end
restore_safe

log ""
log "==========================================="
log "STRESS TEST COMPLETE"
log "Results logged to: $LOG"
log "==========================================="

# Print summary
log ""
log "SUMMARY:"
grep "^.*RESULT:" "$LOG" | while read -r line; do
    log "  $line"
done
