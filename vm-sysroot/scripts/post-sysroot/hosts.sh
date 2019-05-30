sudo tee "$SYSROOT/etc/hosts" >/dev/null << EOF
127.0.0.1	localhost
EOF
sudo touch "$SYSROOT/etc/resolv.conf"
