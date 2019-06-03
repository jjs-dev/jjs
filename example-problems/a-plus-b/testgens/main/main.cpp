#include <cstdio>
#include <cstdlib>

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

int main(int argc, char** argv) {
    int test_id = get_env_int("JJS_TEST_ID");
    int test_out_fd = get_env_int("JJS_TEST");
    FILE* test = fdopen(test_out_fd, "w");
    fprintf(test, "%d %d\n", test_id, test_id * 2 + 1);
}