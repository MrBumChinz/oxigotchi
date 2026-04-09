#!/bin/bash
# PiSugar custom button: long press -> toggle RAGE/SAFE mode
curl -s -X POST http://localhost:8080/api/button -H 'Content-Type: application/json' -d '{"tap":"long"}' > /dev/null 2>&1
