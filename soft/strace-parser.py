import json, ast

while True:
    try: data = input()
    except EOFError: break
    if '(' not in data or ('=' in data and data.find('(') > data.find('=')): continue
    syscall, args = data.split('(', 1)
    syscall = syscall.split()
    if '+++' in syscall: continue
    syscall = syscall[-1]
    args_arr = ['']
    strs_arr = [[]]
    i = iter(args)
    brlev = 0
    for c in i:
        if c in '([{': brlev += 1
        if c in '}])': brlev -= 1
        if brlev < 0: break
        elif c == ',' and brlev == 0:
            args_arr.append('')
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
                elif c2 == '"': break
        else:
            args_arr[-1] += c
    ans = ''.join(i)
    if '=' not in ans: ans = None
    else: ans = ans.split('=', 1)[1].strip()
    if ans == None:
        assert args_arr[-1].endswith('<unfinished ...>')
        args_arr[-1] = args_arr[-1][:-16]
    args_arr = [{'raw': j, 'str_params': [ast.literal_eval('b'+i).decode('latin-1') for i in k]} for j, k in ((j.strip(), k) for j, k in zip(args_arr, strs_arr)) if j]
    print(json.dumps({'syscall': syscall, 'args': args_arr, 'ans': ans}))