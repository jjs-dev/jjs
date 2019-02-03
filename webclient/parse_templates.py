import os

def parse_template(s, p, d):
    s = s.split('\n')
    i = iter(s)
    for l in i:
        if l.startswith('{{#*inline ') or l.startswith('{{#* inline '):
            what = eval(l.split('inline ', 1)[1][:-2])
            data = ''
            while True:
                l = next(i)
                if l == '{{/inline}}':
                    break
                data += '\n' + l
            d[what+'/'+p] = data.strip()

templates = {}

for i in os.listdir('templates'):
    if i.endswith('.hbs'):
        parse_template(open('templates/'+i).read(), i.split('.', 1)[0], templates)

with open('static/app.js', 'w') as file:
    file.write(open('app_template.js').read().replace('TEMPLATES', repr(templates)))
