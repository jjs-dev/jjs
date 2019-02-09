# Installing JJS in virtual machine
## Launch VM
Launch VM.

Make sure you are logged in as a non-root user, and your user can run `sudo` without password prompt.

Also, obtain IP of your host machine
## Prepare host machine
```bash
cd jjs_root
cd devtool
cargo run -- Pkg
cargo run -- Vm
```
## Install jjs
IP = ip-address of host
```bash
curl $IP:4567/setup | bash /dev/stdin $IP
```
## Done!