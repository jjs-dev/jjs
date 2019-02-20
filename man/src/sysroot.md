# Sysroot Layout
* /opt it is a sysroot itself (i.e. it should contain something like `./bin`, `./lib` etc). It's contents will be
exposed to sandbox at both build and run time. It __must not__ contain any files or symlinkks (only directories),
and it __must not__ contain `./workdir` directory 
* /etc contains JJS config files: 
    - /etc/jjs.toml - main config; 
    - /etc/toolchains/*.toml - toolchain configs
* /var/submissions contains submissions info
* /var/submissions/s-<submission_id> contains info for a submission
    - ./source - file with source code
    - ./toolchain toolchain name
    - ./build-workdir pwd when submission was built
    - ./build submission build artifact
    
Sysroot path is referred throughout the manual as $ROOT

## Creating sysroot

__Note__: When running JJS on cluster, make sure sysroot is shared (e.g., using NFS) between all instances.

* Initialize directory structure. You can use `jjs-init-sysroot` CLI utility for it.
* Configure JJS (TODO: page about it). 
* Setup `$ROOT/opt`. You can use `jjs-softinit` CLI utility for it, e.g. `jjs-softinit /tmp/jjs 
--with=g++ --with=gcc`