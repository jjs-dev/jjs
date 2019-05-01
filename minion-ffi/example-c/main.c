#include "minion-ffi.h"
#include <unistd.h>
#include "stdio.h"
const char* MSG_FALLTHROUGH = "unknown error kind";
const char* MSG_INVALIDINPUT = "invalid input";
const char* MSG_UNKNOWN = "unknown error in minion-ffi";
void error_check(int err) {
    if (err == ERROR_CODE_OK) return;
    const char* msg = MSG_FALLTHROUGH;
    if (err == ERROR_CODE_INVALID_INPUT) {
         msg = MSG_INVALIDINPUT;
    } else if (err == ERROR_CODE_UNKNOWN) {
        msg = MSG_UNKNOWN;
    }

    fprintf(stderr, "minon-ffi error %d (%s)\n", err, msg);
    exit(1);
}
int main() {
    int status;
    status = minion_lib_init();
    Minion_Backend* backend;
    minion_backend_create(&backend);
    Minion_DominionOptions dopts;
    dopts.isolation_root = "/tmp/is";
    dopts.process_limit = 1;
    dopts.time_limit.seconds = 1;
    dopts.time_limit.nanoseconds = 0;
    Minion_SharedDirectoryAccess acc;
    acc.kind = SHARED_DIRECTORY_ACCESS_KIND_READONLY;
    acc.host_path = acc.sandbox_path = "/bin";
    dopts.shared_directories = (Minion_SharedDirectoryAccess*) malloc(2*sizeof(acc));
    dopts.shared_directories[0] = acc;
    dopts.shared_directories[1] = SHARED_DIRECTORY_ACCESS_FIN;
    Minion_Dominion* dominion;
    minion_dominion_create(backend, dopts, &dominion);
    Minion_ChildProcessOptions cpopts;
    Minion_ChildProcess* cp;
    minion_cp_spawn(backend, cpopts, &cp);
}