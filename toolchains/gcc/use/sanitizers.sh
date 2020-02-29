#!/usr/bin/env bash
$GCC "$DATA/program.c" -std=c11 -o prog.elf -fsanitize=undefined -fsanitize=address -fsanitize=bounds
