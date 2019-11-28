#!/usr/bin/env bash
$GPP "$DATA/program.cpp" -lm -std=c++17 -o prog.elf -fsanitize=undefined -fsanitize=address -fsanitize=bounds

