# Sysroot Layout
* /opt is a nested sysroot (i.e. it should contain something like `./bin`, `./lib` etc). It's contents will be
exposed to sandbox at both build and run time. It __must not__ contain any files or symlinks (only directories),
and it __must not__ contain `./jjs` directory 
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
* Setup `$ROOT/opt` - toolchain root. 

You can use various strategies to setup toolchain root.
The simplest is to bind-mount / - this is appropriate for testing, but it is unsecure (as a possible attack, hacker can cpp-include /etc/shadow).
Another option is to rebuild all compilers and interpreters from source with appropriate configure options. However, this approach can take much time.
Finally, you can use strace-based toolchain installing. It is secure, fast and easy to use. check `/soft/example-linux.ps1` for details/