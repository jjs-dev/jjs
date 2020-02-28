#!/usr/bin/env bash
$GCC "$DATA/program.c" -std=c18 -o prog.elf
./prog.elf
