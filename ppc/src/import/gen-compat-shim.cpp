// this is thin wrapper for real generator (located in ./main.cpp)

#include "jjs/testgen.h"
#include <unistd.h>

using namespace testgen;

int main(int argc, char** argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: %s path_to_polygon_compatible_test_gen gen_args...", argv[0]);
        return 1;
    }

    Input input = init(false);

    char real_testgen[1024];
    char* dest_dir_path = getenv("JJS_PROBLEM_DEST");
    if (dest_dir_path == nullptr) {
        fprintf(stderr, "error: JJS_PROBLEM_DEST env var is not set");
        return 1;
    }
    sprintf(real_testgen, "%s/assets/module-gen-%s/bin", dest_dir_path, argv[1]);
    argv[1] = real_testgen;

    dup2(input.fd_out_file, 1);

    execv(argv[1], argv + 1);
    fprintf(stderr, "error: execv (path: %s) failed: %m", argv[1]);
    return 1;
}