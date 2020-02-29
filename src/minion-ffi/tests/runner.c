#define _GNU_SOURCE
#include "minion-ffi.h"
#include <assert.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

#define __call_where(f, l, s, ...) f(__FILE__ ":" #l ": `" #s "`", __VA_ARGS__)
#define _call_where(f, l, ...) __call_where(f, l, f(__VA_ARGS__), __VA_ARGS__)
#define call_where(f, ...) _call_where(f, __LINE__, __VA_ARGS__)

static void die(char const* message, ...) {
    va_list l;
    va_start(l, message);
    vfprintf(stderr, "%s", l);
    abort();
}

static inline void verify_ok(char const* where, Minion_ErrorCode code) {
    if (code == ERROR_CODE_OK) {
        return;
    }
    die("%s failed: %s\n", where, minion_describe_status(code));
}

#define verify_ok(...) call_where(verify_ok, __VA_ARGS__)

static inline void assert_write(char const* where, int fd, char const* buf,
                                int count) {
    if (write(fd, buf, count) != count) {
        fprintf(stderr, "%s: write failed: ", where);
        perror("");
        abort();
    }
}

#define assert_write(...) call_where(assert_write, __VA_ARGS__)

#include "tests.h"

void run_test(const char* self, const char* dir, const char* test_name, const struct test* test) {
    int devnull_fd = open("/dev/null", O_RDWR);
    verify_ok(minion_lib_init());
    struct Minion_Backend* bk;
    verify_ok(minion_backend_create(&bk));
    struct Minion_Dominion* sandbox;
    verify_ok(minion_dominion_create(
        bk,
        (struct Minion_DominionOptions) {
            .cpu_time_limit = {test->tl, 0},
            .real_time_limit = {test->il, 0},
            .process_limit = 1,
            .memory_limit = 0x1000000,
            .isolation_root = dir,
            .shared_directories =
                (const struct Minion_SharedDirectoryAccess[5]) {
                    {SHARED_DIRECTORY_ACCESS_KIND_READONLY, self, "/me"},
                    {SHARED_DIRECTORY_ACCESS_KIND_READONLY, "/bin", "/bin"},
                    {SHARED_DIRECTORY_ACCESS_KIND_READONLY, "/lib", "/lib"},
                    {SHARED_DIRECTORY_ACCESS_KIND_READONLY, "/lib64", "/lib64"},
                    SHARED_DIRECTORY_ACCESS_FIN,
                },
        },
        &sandbox));
    struct Minion_ChildProcess* proc;
    verify_ok(
        minion_cp_spawn(bk,
                        (struct Minion_ChildProcessOptions) {
                            .image_path = "/me",
                            .argv = (const char*[]) {test_name},
                            .envp = (struct Minion_EnvItem[1]) {ENV_ITEM_FIN},
                            .stdio = {devnull_fd, dup(1), dup(1)},
                            .dominion = sandbox,
                            .workdir = "/"},
                        &proc));
    Minion_WaitOutcome outcome;
    verify_ok(minion_cp_wait(proc, NULL,
                             &outcome));
    if (outcome == WAIT_OUTCOME_TIMEOUT) {
        bool is_tl, is_il;
        verify_ok(minion_dominion_check_cpu_tle(sandbox, &is_tl));
        verify_ok(minion_dominion_check_real_tle(sandbox, &is_il));
        if (is_tl) {
            assert_write(1, "TL\n", 3);
        } else if (is_il) {
            assert_write(1, "ILE\n", 4);
        } else {
            assert_write(1, "Wait timed out\n", 15);
        }
    } else if (outcome == WAIT_OUTCOME_ALREADY_FINISHED)
        assert_write(1, "Already finished, WTF?\n", 23);
    else {
        int64_t exitcode = 57179444;

        verify_ok(minion_cp_exitcode(proc, &exitcode, NULL));
        char data[64];
        assert_write(1, data,
                     sprintf(data, "exit code %lld\n", (long long) exitcode));
    }
    verify_ok(minion_cp_free(proc));
    verify_ok(minion_dominion_free(sandbox));
    verify_ok(minion_backend_free(bk));
    exit(0);
}

int test_main(int argc, char const* const* argv) {
    if (argc != 2)
        abort();
    for (int i = 0; tests[i].name; i++)
        if (!strcmp(tests[i].name, argv[1])) {
            tests[i].func();
            die("program has not exited after running test");
        }
    fprintf(stderr, "test %s not found", argv[1]);
    return 179;
}

size_t read_all(int fd, const char** buf_p, int* is_timeout) {
    *is_timeout = 0;
    struct timeval timeout = {20, 0};
    char* buf = NULL;
    size_t cap = 0;
    size_t sz = 0;
    while (1) {
        fd_set fds;
        FD_ZERO(&fds);
        FD_SET(fd, &fds);
        if(!select(fd+1, &fds, NULL, NULL, &timeout))
        {
            *is_timeout = 1;
            break;
        }
        if (sz == cap) {
            cap = 2 * cap + 1;
            buf = realloc(buf, cap);
            assert(buf);
        }
        ssize_t chunk_sz = read(fd, buf + sz, cap - sz);
        assert(chunk_sz >= 0);
        if (chunk_sz == 0) {
            break;
        }
        sz += chunk_sz;
    }
    *buf_p = buf;
    return sz;
}

int main(int argc, const char** argv) {
    if (argc == 2) {
        return test_main(argc, argv);
    }
    char self[1024];
    int sz = readlink("/proc/self/exe", self, 1022);
    if (sz < 0) {
        die("readlink failed");
    }
    self[sz] = 0;
    if (argc != 1) {
        fprintf(stderr, "usage: sudo %s\n\nRun minion-ffi tests.", argv[0]);
        return 2;
    }
    int have_fails = 0;
    for (int i = 0; tests[i].name; i++) {
        fprintf(stderr, "running `%s`\n", tests[i].name);
        int fail = 0;
        char tempdir[] = "/tmp/tmpXXXXXX";
        assert(mkdtemp(tempdir));
        int comm_pipe[2];
        assert(!pipe(comm_pipe));
        int devnullfd = open("/dev/null", O_RDONLY);
        assert(devnullfd >= 0);
        int pid = fork();
        if (pid == -1) {
            perror("fork");
            die("");
        }
        if (!pid) {
            close(comm_pipe[0]);
            assert(dup2(devnullfd, 0) == 0);
            assert(dup2(comm_pipe[1], 1) == 1);
            run_test(self, tempdir, tests[i].name, &tests[i]);
            die("program has not exited during run_test");
        }
        close(comm_pipe[1]);
        close(devnullfd);
        const char* output;
        int is_timeout;
        size_t output_sz = read_all(comm_pipe[0], &output, &is_timeout);
        const char* output0 = output;
        size_t expected_output_sz = strlen(tests[i].expected_output);
        if (output_sz != expected_output_sz ||
            memcmp(output, tests[i].expected_output, output_sz) ||
            is_timeout) {
            fail = 1;
            fprintf(stderr, "test `%s`: ", tests[i].name);
            if(is_timeout)
                fprintf(stderr, "timeout");
            else
                fprintf(stderr, "output differs");
            fprintf(stderr,
                    ":\nActual output (len %lld):\n",
                    (long long) output_sz);
            while (output_sz) {
                ssize_t chunk_sz = fwrite(output, 1, output_sz, stderr);
                assert(chunk_sz >= 0);
                output += chunk_sz;
                output_sz -= chunk_sz;
            }
            fprintf(stderr, "\nExpected output (len %lld):\n%s\n",
                    (long long) expected_output_sz, tests[i].expected_output);
        }
        free((void*) output0);
        int status;
        int q = waitpid(pid, &status, WNOHANG);
        assert(q == 0 || q == pid);
        if(q == 0)
        {
            kill(pid, SIGKILL);
            die("*** FATAL ERROR: test `%s`: timeout. The process will "
                "be killed. Please kill the remaining processes (if any) "
                "and clean the tempdir (%s) MANUALLY! ***\n",
                tests[i].name, tempdir);
        }
        if (!WIFEXITED(status) || WEXITSTATUS(status)) {
            die("*** FATAL ERROR: test `%s`: run_test aborted. Please clean "
                "the tempdir (%s) MANUALLY! ***\n",
                tests[i].name, tempdir);
        }
        if (fail) {
            fprintf(stderr, "test `%s` FAIL\n", tests[i].name);
            have_fails = 1;
        } else {
            fprintf(stderr, "test `%s` OK\n", tests[i].name);
        }
        pid = fork();
        if (!pid) {
            execlp("rm", "rm", "-r", tempdir, NULL);
            perror("rm -rf $tempdir");
            abort();
        }
        assert(wait(&status) == pid);
        if (!WIFEXITED(status) || WEXITSTATUS(status)) {
            die("rm -rf $tempdir");
        };
    }
    if (!have_fails) {
        fprintf(stderr, "all OK\n");
    }
    return have_fails;
}
