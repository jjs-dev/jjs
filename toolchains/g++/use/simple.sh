#!/usr/bin/env bash
$GPP "$DATA/program.cpp" -lm -std=c++17 -o prog.elf
./prog.elf
