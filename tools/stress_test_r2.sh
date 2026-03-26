#!/bin/bash
# Oxigotchi Firmware Stress Test - Round 2
# Push absolute limits: Rate 3 with low dwell, all channels, worst-case combos
# Also test Rate 2 with minimum dwell + all channels (gap from round 1)
#
# Safety: monitors for hard crashes, restores safe config on failure

API="http://localhost:8080"
TEST_DURATION=120  # 2 minutes per combo
SAFE_RATE=1
SAFE_DWELL=5000
SAFE_CHANNELS='[1,6,11]'
LOG="/tmp/stress_test_r2_$(date +%Y%m%d_%H%M%S).log"

log() { echo "[$(date '+%H:%M:%S')] $*" | tee -a "$LOG"; }

count_psm_since() {
    local start_line=$1
    local n
    n=$(dmesg | tail -n +"$start_line" | grep -c "PSM's watchdog has fired" 2>/dev/null) || true
    echo "${n:-0}" | tr -d '[:space:]'
}

ao_alive() {
    pgrep -f "angryoxide.*--interface" >/dev/null 2>&1
}

check_hard_crash() {
    local start_line=$1
    local n
    n=$(dmesg | tail -n +"$start_line" | grep -cE "bus is down|error.*-110|Set Channel failed.*-110|brcmf_sdio_bus_rxctl" 2>/dev/null) || true
    echo "${n:-0}" | tr -d '[:space:]'
}

wlan_alive() {
    ip link show wlan0mon >/dev/null 2>&1
}

restore_safe() {
    log "RESTORING safe config: rate=$SAFE_RATE dwell=$SAFE_DWELL channels=$SAFE_CHANNELS"
    curl -s -X POST "$API/api/rate" -H 'Content-Type: application/json' -d "{\"rate\": $SAFE_RATE}" >/dev/null 2>&1
    sleep 2
    curl -s -X POST "$API/api/channels" -H 'Content-Type: application/json' \
        -d "{\"channels\": $SAFE_CHANNELS, \"dwell_ms\": $SAFE_DWELL, \"autohunt\": false}" >/dev/null 2>&1
    sleep 5
}

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

    local dmesg_lines
    dmesg_lines=$(dmesg | wc -l | tr -d '[:space:]')

    # Apply rate
    curl -s -X POST "$API/api/rate" -H 'Content-Type: application/json' \
        -d "{\"rate\": $rate}" >/dev/null 2>&1
    sleep 3

    # Apply channel config
    curl -s -X POST "$API/api/channels" -H 'Content-Type: application/json' \
        -d "{\"channels\": $channels, \"dwell_ms\": $dwell_ms, \"autohunt\": $autohunt}" >/dev/null 2>&1
    sleep 3

    if ! ao_alive; then
        log "RESULT: FAIL - AO died immediately after config change"
        restore_safe
        sleep 10
        return 1
    fi

    local elapsed=0
    local check_interval=10
    local ao_died=false
    local hard_crash=false
    local wlan_gone=false

    while [ $elapsed -lt $TEST_DURATION ]; do
        sleep $check_interval
        elapsed=$((elapsed + check_interval))

        if ! ao_alive; then
            log "  [${elapsed}s] AO process died!"
            ao_died=true
            break
        fi

        if ! wlan_alive; then
            log "  [${elapsed}s] wlan0mon disappeared!"
            wlan_gone=true
            break
        fi

        local hc
        hc=$(check_hard_crash "$dmesg_lines")
        if [ "${hc:-0}" -gt 0 ] 2>/dev/null; then
            log "  [${elapsed}s] Hard crash detected ($hc events)"
            hard_crash=true
            break
        fi

        if [ $((elapsed % 30)) -eq 0 ]; then
            local psm
            psm=$(count_psm_since "$dmesg_lines")
            log "  [${elapsed}s] OK - PSM fires: ${psm:-0}, AO: alive, wlan: up"
        fi
    done

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
        sleep 15  # longer cooldown after crash
        return 1
    else
        log "RESULT: PASS (${TEST_DURATION}s)"
        log "  PSM watchdog fires: $psm_total"
        log "  Hard crash events: $hc_total"
    fi

    sleep 5
    return 0
}

# ============================================================
log "Oxigotchi Firmware Stress Test - ROUND 2 (Extreme)"
log "Duration per test: ${TEST_DURATION}s"
log "Goal: find the breaking point"
log ""

restore_safe
sleep 5

# === Fill gaps from Round 1 ===
# R2 with low dwell + all channels (not tested in R1)
run_test "R2-D2000-ALL13"     2 2000 '[1,2,3,4,5,6,7,8,9,10,11,12,13]'
run_test "R2-D1000-ALL13"     2 1000 '[1,2,3,4,5,6,7,8,9,10,11,12,13]'
run_test "R2-D500-ALL13"      2  500 '[1,2,3,4,5,6,7,8,9,10,11,12,13]'

# === Rate 3 all channels, decreasing dwell ===
run_test "R3-D8000-ALL13"     3 8000 '[1,2,3,4,5,6,7,8,9,10,11,12,13]'
run_test "R3-D5000-ALL13"     3 5000 '[1,2,3,4,5,6,7,8,9,10,11,12,13]'
run_test "R3-D3000-ALL13"     3 3000 '[1,2,3,4,5,6,7,8,9,10,11,12,13]'
run_test "R3-D2000-ALL13"     3 2000 '[1,2,3,4,5,6,7,8,9,10,11,12,13]'
run_test "R3-D1000-ALL13"     3 1000 '[1,2,3,4,5,6,7,8,9,10,11,12,13]'
run_test "R3-D500-ALL13"      3  500 '[1,2,3,4,5,6,7,8,9,10,11,12,13]'

# === Rate 3 with few channels, minimum dwell (max attack density) ===
run_test "R3-D2000-CH1611"    3 2000 '[1,6,11]'
run_test "R3-D1000-CH1611"    3 1000 '[1,6,11]'
run_test "R3-D500-CH1611"     3  500 '[1,6,11]'

# === Autohunt mode at extreme settings ===
run_test "R3-D2000-AUTOHUNT"  3 2000 '[]' true
run_test "R2-D500-AUTOHUNT"   2  500 '[]' true

# Always restore safe at the end
restore_safe

log ""
log "==========================================="
log "STRESS TEST ROUND 2 COMPLETE"
log "Results logged to: $LOG"
log "==========================================="
log ""
log "SUMMARY:"
grep "RESULT:" "$LOG" | while read -r line; do
    log "  $line"
done
