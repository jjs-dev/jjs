// this is thin wrapper for real generator (located in ./main.cpp)

#include "jjs/testgen.h"
#include <unistd.h>

using namespace testgen;

int main(int argc, char** argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s path_to_polygon_compatible_test_gen", argv[0]);
    }

    Input input = init();

    dup2(1, input.fd_out_file);
    execl(argv[1], argv[1], nullptr);
}