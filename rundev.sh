#!/usr/bin/env bash
./dev.sh /bin/bash -x /bin:r-x:/bin -x /lib64:r-x:/lib64 -x /lib:r-x:/lib -x /usr:r-x:/usr \
    -x /etc:r-x:/etc -x /out:rwx:/shr -x /sbin:r-x:/sbin