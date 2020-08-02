#include <cassert>
#include <wait.h>

#include "checker.h"
#include "unistd.h"
#include "util.h"

using namespace checker;

static const size_t PATH_LEN = 128;

int main(int argc, char** argv) {
    if (argc != 2) {
        fprintf(stderr, "Usage: %s path_to_polygon_compatible_checker",
                argv[0]);
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
    int fres = fork();
    if (fres == -1) {
        fprintf(stderr, "fork() failed: %m\n");
        return 1;
    }
    if (fres == 0) {
        execl(inner_checker, inner_checker, input_file, output_file,
              answer_file, nullptr);
        fprintf(stderr, "error: launch inner checker %s: %d (%m)\n",
                inner_checker, errno);
        exit(66);
    }
    int wstatus;
    if (waitpid(fres, &wstatus, 0) == -1) {
        fprintf(stderr, "error: waitpid() failed: %m\n");
        exit(1);
    }
    if (WIFEXITED(wstatus)) {
        int exit_code = WEXITSTATUS(wstatus);
        switch (exit_code) {
        case 0:
            finish(Outcome::OK);
            break;
        case 1:
            finish(Outcome::WRONG_ANSWER);
            break;
        case 2:
        case 4:
        case 8:
            finish(Outcome::PRESENTATION_ERROR);
            break;
        case 3:
            finish(Outcome::CHECKER_LOGIC_ERROR);
            break;
        default: {
            // fallthrough
        }
        }
        fprintf(stderr, "unexpected return code from child checker: %d\n",
                exit_code);
        exit(1);
    } else {
        fprintf(
            stderr,
            "unexpected exit status (child checker didn't terminate normally): "
            "%d\n",
            wstatus);
        exit(1);
    }
}