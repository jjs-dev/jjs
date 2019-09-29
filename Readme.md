[![Bors enabled](https://bors.tech/images/badge_small.svg)](https://app.bors.tech/repositories/20068)

# JJS
Judging system

## Quick start

```bash
# Note: while installation process is intentionally made similar to that of most other Linux tools, JJS doesn't use autotools

# Of course, you can build out-of-tree
# Note for contributors: it is recommended to build in target
mkdir build && cd build

# This will automatically include most of JJS features. See ../configure --help for possible options
../configure --out /opt/jjs

# Install dependencies
make deps

## Alternatively, you can do it manually:
# sudo apt-get install libpq-dev
# sudo apt-get install libssl-dev
# cargo install cbindgen
# cargo install mdbook

# Now, start build. This will also install JJS
# If you don't have make installed, run ./make
make

# Done. JJS is now installed
# Don't forget to include some env vars:
. /opt/jjs/share/env.sh
```

## License
Licensed under either of
- [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
- [MIT license](http://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as 
defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
