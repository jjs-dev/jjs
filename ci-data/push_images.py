#!/usr/bin/env python3
import subprocess

TEMPLATES = [
    "docker.pkg.github.com/jjs-dev/jjs/jjs-%:latest",
    "gcr.io/jjs-dev/jjs-%:latest"
]

tags = open("/tmp/taglog.txt").readlines()
all_images = []
for comp in tags:
    comp = comp.strip()
    for tpl in TEMPLATES:
        new_tag = tpl.replace("%", comp)
        subprocess.check_call(["docker", "tag", comp, new_tag])
        all_images.append(new_tag)
print("will push", all_images)

for img in all_images:
    subprocess.check_call(["docker", "push", img])
