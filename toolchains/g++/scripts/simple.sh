#!/usr/bin/env bash
g++ "$DATA/program.cpp" -lm -std=c++17 -o prog.elf
./prog.elf
