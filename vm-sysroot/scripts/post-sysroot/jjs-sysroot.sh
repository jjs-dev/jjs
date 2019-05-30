ORIG_CWD="$(pwd)"

sudo mkdir -p "$SYSROOT/var/lib/jjs"
sudo chown "$(whoami):$(whoami)" "$SYSROOT/var/lib/jjs"
cd ../init-jjs-root
cargo run -- "$SYSROOT/var/lib/jjs" ../pkg/ar_data/example-config

sudo chown -R 1:1 "$SYSROOT"/var/lib/jjs/*
sudo chown root:root "$SYSROOT/var/lib/jjs"
sudo chmod -R 0700 "$SYSROOT"/var/lib/jjs/*
sudo chmod 0755 "$SYSROOT"/var/lib/jjs/var{,/submissions}

sudo rm -rf "$SYSROOT/var/lib/jjs/opt"
sudo mkdir "$SYSROOT/var/lib/jjs/opt"
pwsh "$ORIG_CWD/invoker-sysroot.ps1" "$SYSROOT/var/lib/jjs/opt"
