#!/usr/bin/env bash

sudo mkdir -p "$SYSROOT/usr/bin"
sudo tee "$SYSROOT/usr/bin/jjs-prod" >/dev/null << EOF
#!/bin/sh

if ! grep -q '^export JJS_ENV=dev$' /init
then if ! grep -q '^export JJS_ENV=prod$' /init
    then
        echo '/init has been modified, exiting'
        exit 1
    fi
    echo 'Already running in production mode.'
    exit 0
fi
echo 'Generating JJS_SECRET_KEY...'
sed -i 's/^#export JJS_SECRET_KEY=$/export JJS_SECRET_KEY='"\$(dd if=/dev/random bs=64 count=1 | xxd -p -c 64)"'/g' /init
echo 'Setting JJS_ENV=prod'
sed -i 's/^export JJS_ENV=dev$/export JJS_ENV=prod/g' /init
echo 'Done setting up. Reboot or restart JJS manually to apply changes.'
EOF
sudo chmod +x "$SYSROOT/usr/bin/jjs-prod"
