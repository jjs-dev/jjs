#!/bin/python3

import json
import ast
import os
import sys
import errno
lxtrace_compat = False
input_file_name = None
for arg in sys.argv:
    if arg == '--lxtrace':
        lxtrace_compat = True
    elif arg[:len("--input")] == "--input":
        input_file_name = arg[len("--input="):]
if input_file_name is not None:
    sys.stdin = open(input_file_name)
while True:
    try:
        data = input()
    except EOFError:
        break
    if '(' not in data or ('=' in data and data.find('(') > data.find('=')):
        continue
    syscall, args = data.split('(', 1)
    if '<' in syscall or '+++' in syscall:
        continue
    if '+++' in syscall:
        continue
    syscall = syscall.split()
    if len(syscall) > 1 and syscall[0].isnumeric():
        pid = int(syscall[0])
    else:
        pid = None
    syscall = syscall[-1]
    args_arr = ['']
    unk_arr = ['']
    strs_arr = [[]]
    i = iter(args)
    brlev = 0
    for c in i:
        if c in '([{':
            brlev += 1
        if c in '}])':
            brlev -= 1
        if brlev < 0:
            break
        elif c == ',' and brlev == 0:
            args_arr.append('')
            unk_arr.append('')
            strs_arr.append([])
        elif c == '"':
            args_arr[-1] += c
            strs_arr[-1].append(c)
            for c2 in i:
                args_arr[-1] += c2
                strs_arr[-1][-1] += c2
                if c2 == '\\':
                    args_arr[-1] += next(i)
                    strs_arr[-1][-1] += args_arr[-1][-1]
                elif c2 == '"':
                    break
        else:
            args_arr[-1] += c
            unk_arr[-1] += c
    ans = ''.join(i)
    if '=' not in ans:
        ans = None
    else:
        ans = ans.split('=', 1)[1].strip()
    if ans == None:
        assert args_arr[-1].endswith('<unfinished ...>'), (args_arr, data)
        args_arr[-1] = args_arr[-1][:-16]
    args_arr = [{'raw': j, 'str_params': [ast.literal_eval(
        'b'+i).decode('latin-1') for i in k]} for j, k in ((j.strip(), k) for j, k in zip(args_arr, strs_arr)) if j]
    if lxtrace_compat:
        compat_args = []
        for i, u in zip(args_arr, unk_arr):
            raw = i['raw']
            if raw == 'NULL':
                compat_args.append({'kind': 'address', 'data': 0})
                continue
            if raw.lower().startswith('0x'):
                try:
                    compat_args.append(
                        {'kind': 'address', 'data': int(raw, 16)})
                except ValueError:
                    pass
                else:
                    continue
            try:
                compat_args.append({'kind': 'integral', 'data': int(
                    raw, 8 if raw.isnumeric() and raw.startswith('0') else 0)})
            except ValueError:
                pass
            else:
                continue
            u = u.strip()
            if u in ('', '...') and len(i['str_params']) == 1:
                compat_args.append(
                    {'kind': 'string', 'data': i['str_params'][0]})
            elif u == '['+', '*(len(i['str_params']) - 1)+']':
                compat_args.append(
                    {'kind': 'string_array', 'data': i['str_params']})
            else:
                i2 = dict(i)
                i2['kind'] = 'unknown'
                compat_args.append(i2)
        compat_ret = None
        if ans != None:
            if ans.startswith('-1 '):
                code = getattr(errno, 'E'+ans.split('E', 1)[1].split()[0])
                compat_ret = {'kind': 'error',
                              'data': [code, os.strerror(code)]}
            else:
                try:
                    if not ans.lower().startswith('0x'):
                        raise ValueError("not hex")
                    compat_ret = {'kind': 'address', 'data': int(ans, 16)}
                except ValueError:
                    try:
                        compat_ret = {'kind': 'integral', 'data': int(
                            ans, 8 if ans.isnumeric() and ans.startswith('0') else 0)}
                    except ValueError:
                        compat_ret = {'kind': 'unknown', 'raw': ans}
        decoded = {
            'name': syscall,
            'args': compat_args,
            'ret': compat_ret
        }
        data = {
            'decoded': decoded
        }
        print(json.dumps(
            {'payload': {'kind': 'sysenter', 'data': data}, 'pid': pid}))
    else:
        print(json.dumps({'syscall': syscall, 'args': args_arr, 'ans': ans}))
