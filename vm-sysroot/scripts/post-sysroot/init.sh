sudo mkdir "$SYSROOT"/{dev,proc,sys}

sudo tee "$SYSROOT/init" >/dev/null << EOF
#!/bin/sh

mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devpts /dev/pts
mkdir -p /dev/shm
chmod 777 /dev/shm

mount -t tmpfs tmpfs /sys/fs/cgroup
for i in cpuacct pids memory
do
    mkdir /sys/fs/cgroup/\$i
    mount -t cgroup -o nosuid,nodev,noexec,\$i cgroup /sys/fs/cgroup/\$i
done
echo 1 > /sys/fs/cgroup/memory/memory.use_hierarchy

mount -o remount,rw /
ifdown lo
ifup lo

su postgres -c 'postgres -D /var/lib/postgresql/*/main &'
sleep 5

echo "We are: \$(id)"

su jjs -c '
export JJS_SYSROOT=/var/lib/jjs
export DATABASE_URL=postgres://jjs:internal@localhost:5432/jjs
export RUST_BACKTRACE=1
export JJS_HOST=0.0.0.0
jjs-frontend &
'

export JJS_SYSROOT=/var/lib/jjs
export DATABASE_URL=postgres://jjs:internal@localhost:5432/jjs
export RUST_BACKTRACE=1
jjs-invoker &

ifdown eth0
ifup eth0

if [ "$$" == 1 ]
then
sh
killall jjs-frontend
killall jjs-invoker
killall -INT postgres
while killall -0 postgres
do true
done
mount -o remount,sync /
mount -o remount,ro /
sync
poweroff -f
fi
EOF
sudo chmod +x "$SYSROOT/init"
sudo mkdir -p "$SYSROOT/etc/init.d"
sudo ln -s /init "$SYSROOT/etc/init.d/rcS"
