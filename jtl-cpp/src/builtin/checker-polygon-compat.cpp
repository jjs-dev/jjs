#include <cassert>

#include "checker.h"
#include "util.h"
#include "unistd.h"

using namespace checker;

static const size_t PATH_LEN = 64;

int main(int argc, char** argv) {
    if (argc != 2) {
        fprintf(stderr, "Usage: %s path_to_polygon_compatible_checker", argv[0]);
        exit(1);
    }
    CheckerInput input = init(false);
    char input_file[PATH_LEN];
    char output_file[PATH_LEN];
    char answer_file[PATH_LEN];

    pid_t my_pid = getpid();
    assert(my_pid != -1);

    sprintf(input_file, "/proc/%d/fd/%d", my_pid, (int) input.fd_test);
    sprintf(output_file, "/proc/%d/fd/%d", my_pid, (int) input.fd_sol);
    sprintf(answer_file, "/proc/%d/fd/%d", my_pid, (int) input.fd_corr);

    char* inner_checker = argv[1];

    if (execl(inner_checker, inner_checker, input_file, output_file, answer_file, nullptr) == -1) {
        fprintf(stderr, "error: launch inner checker: %d (%m)", errno);
    }
}