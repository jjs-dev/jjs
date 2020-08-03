#!/usr/bin/env python3

import json

ans = set()

while True:
    try:
        l = json.loads(input())
    except EOFError:
        break
    if l['syscall'] in ('open', 'execve'):
        path = l['args'][0]['str_params'][0]
    elif l['syscall'] in ('openat',):
        path = l['args'][1]['str_params'][0]
    else:
        continue
    if path.startswith('/') and all(not path.startswith(i) for i in ('/dev', '/proc', '/sys')):
        ans.add(path)

for i in ans:
    print(i)
