#include "minion-ffi.h"
#include <unistd.h>
int main() {
    minion_lib_init();
    Minion_Backend* backend = minion_setup();
    Minion_DominionOptionsWrapper* options = minion_dominion_options_create();
    minion_dominion_options_isolation_root(options, "/tmp/is");
    minion_dominion_options_process_limit(options, 1);
    minion_dominion_options_time_limit(options, 1, 0);
    minion_dominion_options_expose_path(options, "/bin", "/bin", SHARED_DIRECTORY_ACCESS_READ_ONLY);
    Minion_DominionWrapper* dominion = minion_dominion_create(backend, options);
    Minion_ChildProcessOptionsWrapper* cp_options = minion_cp_options_create(dominion);
    minion_cp_options_set_image_path(cp_options, "/bin/echo");

}