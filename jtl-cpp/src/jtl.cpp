#include "jtl.h"
#include <cstdlib>
#include <cstdio>

char* get_env(const char* var_name) {
    char* res = getenv(var_name);
    if (res == nullptr) {
        fprintf(stderr, "ERROR: var %s not present\n", var_name);
        exit(1);
    }
    return res;
}

int get_env_int(const char* var_name) {
    char* res = get_env(var_name);
    int ans;
    if (sscanf(res, "%d", &ans) == 0) {
        fprintf(stderr, "ERROR: var `%s` has value `%s`, which is not integer\n", var_name, res);
        exit(1);
    }
    return ans;
}

FILE* get_env_file(const char* var_name, const char* mode) {
    int fd = get_env_int(var_name);
    FILE* file = fdopen(fd, mode);
    if (file == nullptr) {
        fprintf(stderr, "ERROR: var `%s` contains fd `%d`, which is not file of mode %s", var_name, fd, mode);
        exit(1);
    }
    return file;
}

TestgenInput init_testgen() {
    TestgenInput ti;
    ti.test_id = get_env_int("JJS_TEST_ID");
    ti.out_file = get_env_file("JJS_TEST", "w");
    return ti;
}