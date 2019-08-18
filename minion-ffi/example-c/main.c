#include "minion-ffi.h"
#include <unistd.h>
#include "stdio.h"
#include "assert.h"

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

    fprintf(stderr, "minion-ffi error %d (%s)\n", err, msg);
    exit(1);
}

int main() {
    int status;
    status = minion_lib_init();
    error_check(status);
    struct Minion_Backend* backend;
    status = minion_backend_create(&backend);
    error_check(status);
    struct Minion_DominionOptions dopts;
    dopts.isolation_root = "/tmp/is";
    dopts.process_limit = 1;
    dopts.time_limit.seconds = 1;
    dopts.time_limit.nanoseconds = 0;
    struct Minion_SharedDirectoryAccess acc;
    acc.kind = SHARED_DIRECTORY_ACCESS_KIND_READONLY;
    acc.host_path = acc.sandbox_path = "/bin";
    dopts.shared_directories = (struct Minion_SharedDirectoryAccess*) malloc(2 * sizeof(acc));
    assert(dopts.shared_directories);
    dopts.shared_directories[0] = acc;
    dopts.shared_directories[1] = SHARED_DIRECTORY_ACCESS_FIN;
    struct Minion_Dominion* dominion;
    status = minion_dominion_create(backend, dopts, &dominion);
    error_check(status);
    struct Minion_ChildProcessOptions cpopts;
    cpopts.dominion = dominion;
    cpopts.stdio.stdout = 1;
    cpopts.argv = (char**) malloc(2 * sizeof(char*));
    assert(cpopts.argv);
    cpopts.argv[0] = "ls";
    cpopts.argv[1] = NULL;
    cpopts.envp = (struct Minion_EnvItem*) malloc(sizeof(struct Minion_EnvItem));
    assert(cpopts.envp);
    cpopts.envp[0] = ENV_ITEM_FIN;
    struct Minion_ChildProcess* cp;
    status = minion_cp_spawn(backend, cpopts, &cp);
    error_check(status);
}