# Installing JJS in Virtual Machine
## Launch VM
- Launch VM.
- Make sure you are logged in as a non-root user, and your user can run `sudo` without password prompt.
- Obtain IP of your host machine as it seen from guest.
## Prepare host machine
```bash
cd jjs_root
cd devtool
cargo run -- pkg
cargo run -- pm
```
## Install jjs
IP = ip-address of host
```bash
curl $IP:4567/setup | bash /dev/stdin $IP
```
## Done!