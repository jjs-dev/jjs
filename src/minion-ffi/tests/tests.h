#include <unistd.h>

void test_tl();

void test_tl_fork();

void test_il();

void test_abort();

void test_return_1();

void test_ok();

void test_consume_memory();

struct test {
    const char* name;
    void (*func)(void);
    const char* expected_output;
    int tl;
    int il;
};

const extern struct test tests[];
