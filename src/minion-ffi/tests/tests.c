#include "tests.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <signal.h>
static void exceed_time_limit() {
    while (1) {
        // just some staff with side effects
        write(-1 /*invalid FD*/, "", 0);
    }
}
void test_tl() { exceed_time_limit(); }

void test_tl_fork() {
    fork();
    exceed_time_limit();
}

void test_il() { usleep(10000000); }

void test_abort() { abort(); }

void test_return_1() { exit(1); }

void test_ok() { exit(0); }

void test_consume_memory() {
    // alloc 1 GiB
    size_t const allocation_size = ((size_t) 1) << 30;
    char* ptr = (char*) malloc(allocation_size);
    if (ptr == NULL) {
        printf("OOM\n");
        kill(0, SIGKILL);
    }
    memset(ptr, 0, allocation_size);
    size_t const page_size = 4096;
    unsigned int cnt = 0;
    for (int i = 0; i < 10000; ++i) {
        int j = rand() % allocation_size;
        ptr[j] = j;
        cnt += j;
        int j2 = rand() % allocation_size;
        cnt += ptr[j2];
    }
    printf("did not fail: %d\n", (int) cnt);
    exit(0);
}

const struct test tests[] = {
    {"tl", test_tl, "TL\n", 1, 2},
    {"tl_fork", test_tl_fork, "TL\n", 1, 2},
    {"il", test_il, "ILE\n", 1, 2},
    {"abort", test_abort, "exit code -6\n", 1, 2},
    {"return1", test_return_1, "exit code 1\n", 1, 2},
    {"ok", test_ok, "exit code 0\n", 1, 2},
    {"consume_memory", test_consume_memory, "exit code -9\n", 10, 25},
    {"wait_timeout", test_il, "Wait timed out\n", 1, 10},
    {NULL, NULL, NULL}};
