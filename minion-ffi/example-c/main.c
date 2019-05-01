#include "minion-ffi.h"
#include <unistd.h>
int main() {
    minion_lib_init();
    Minion_Backend* backend;
    minion_backend_create(&backend);
    Minion_DominionOptions dopts;
    dopts.isolation_root = "/tmp/is";
    dopts.process_limit = 1;
    dopts.time_limit.seconds = 1;
    dopts.time_limit.nanoseconds = 0;
    Minion_SharedDirectoryAccess acc;
    acc.kind = SHARED_DIRECTORY_ACCESS_KIND_READ_ONLY;
    acc.host_path = acc.sandbox_path = "/bin";
    dopts.shared_directories = (Minion_SharedDirectoryAccess*) malloc(2*sizeof(acc));
    dopts.shared_directories[0] = &acc;
    dopts.shared_directories[1] = NULL;
    Minion_DominionWrapper* dominion = minion_dominion_create(backend, dopts);
    Minion_ChildProcessOptionsWrapper* cp_options = minion_cp_options_create(dominion);
    minion_cp_options_set_image_path(cp_options, "/bin/echo");
}