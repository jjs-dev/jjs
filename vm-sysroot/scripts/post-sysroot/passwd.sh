sudo mkdir -p "$SYSROOT/etc"
sudo tee "$SYSROOT/etc/passwd" >/dev/null << EOF
root:x:0:0:root:/:/bin/sh
jjs:x:1:1:jjs:/:/bin/sh
postgres:x:2:2:postgres:/:/bin/sh
EOF
sudo tee "$SYSROOT/etc/group" >/dev/null << EOF
root:x:0:
jjs:x:1:
postgres:x:2:
EOF
