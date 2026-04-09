#!/bin/bash
# PiSugar custom button: single tap -> cycle rage level
curl -s -X POST http://localhost:8080/api/button -H 'Content-Type: application/json' -d '{"tap":"single"}' > /dev/null 2>&1
