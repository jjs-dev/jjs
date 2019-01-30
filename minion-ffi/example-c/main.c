#include "minion_ffi.h"
#include <unistd.h>
int main() {
    minion_lib_init();
    Minion_Backend* backend = minion_setup();
    Minion_DominionOptionsWrapper* options = minion_dominion_options_create();
    minion_dominion_options_isolation_root(options, "/tmp/is");
    minion_dominion_options_process_limit(options, 1);
    minion_dominion_options_time_limit(options, 1, 0);
    Minion_DominionWrapper* dominion = minion_dominion_create(backend, options);
}