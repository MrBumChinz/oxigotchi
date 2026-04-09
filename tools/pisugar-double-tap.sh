#!/bin/bash
# PiSugar custom button: double tap -> toggle BT tethering
curl -s -X POST http://localhost:8080/api/button -H 'Content-Type: application/json' -d '{"tap":"double"}' > /dev/null 2>&1
