sudo mkdir -p "$SYSROOT/etc/network"
sudo cp etc-network-interfaces.conf "$SYSROOT/etc/network/interfaces"
sudo chown root:root "$SYSROOT/etc/network/interfaces"
