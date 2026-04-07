#!/bin/bash
# PiSugar single-tap: toggle between SAFE mode (BT tether) and RAGE mode
MODE=$(curl -s http://localhost:8080/api/status 2>/dev/null \
  | grep -o '"mode":"[^"]*"' | grep -o '"[^"]*"$' | tr -d '"')
if [ "$MODE" = "SAFE" ] || [ "$MODE" = "BT" ]; then
    curl -s -X POST http://localhost:8080/api/mode \
      -H 'Content-Type: application/json' \
      -d '{"mode":"RAGE"}' || true
else
    curl -s -X POST http://localhost:8080/api/mode \
      -H 'Content-Type: application/json' \
      -d '{"mode":"SAFE"}' || true
fi
