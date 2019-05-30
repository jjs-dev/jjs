sudo mkdir -p "$SYSROOT/bin"
sudo cp /bin/busybox "$SYSROOT/bin"
busybox --list-full | while read applet
do
    if [ ! -e "$SYSROOT/$applet" ]
    then
        sudo mkdir -p "$SYSROOT/$applet"
        sudo rmdir "$SYSROOT/$applet"
        sudo ln -s /bin/busybox "$SYSROOT/$applet"
    fi
done
