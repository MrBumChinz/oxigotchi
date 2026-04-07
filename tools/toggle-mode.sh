#!/bin/bash
# PiSugar double-tap: cycle operating mode (RAGE → BT → SAFE → RAGE)
curl -s -X POST http://localhost:8080/api/mode \
  -H 'Content-Type: application/json' \
  -d '{"mode":"TOGGLE"}' || true
