#!/usr/bin/env python3

import sys
if len(sys.argv) != 3:
    print(f"Usage: {sys.argv[0]} <path_to_strace_file> <path_to_out_file>")
    exit(1)

total_lines = 0
filter_sigill = 0
accepted_lines = 0

fin = open(sys.argv[1])
fout = open(sys.argv[2], 'w')

sigilled = set()

for line in fin:
    total_lines += 1
    pid = line.split()[0]
    pid = int(pid)
    is_sigill = "--- SIGILL {si_signo=SIGILL, si_code=ILL_ILLOPN," in line or "--- SIGSEGV {si_signo=SIGSEGV" in line
    if is_sigill:
        if pid in sigilled:
            filter_sigill += 1
            continue
        else:
            sigilled.add(pid)
    accepted_lines += 1
    fout.write(line)

total_filtered = total_lines - accepted_lines

print(f"Got {total_lines} lines, {total_filtered} filtered out, {accepted_lines} included in report")
print("Filter summary")
print(f"\t duplicate SIGs: {filter_sigill}")
