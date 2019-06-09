#include "jtl.h"
#include <cstdlib>
#include <cstdio>
#include <cstdarg>
#include <cctype>
#include <cassert>

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

struct CheckerData {
    CheckerInput inp;
    FILE* out_file;
    FILE* comment_file;
};

CheckerData CHECKER;

CheckerInput init_checker() {
    CheckerInput inp;
    inp.corr_answer = get_env_file("JJS_CORR", "r");
    inp.sol_answer = get_env_file("JJS_SOL", "r");
    inp.test = get_env_file("JJS_TEST", "r");
    CHECKER.out_file = get_env_file("JJS_CHECKER_OUT", "w");
    CHECKER.comment_file = get_env_file("JJS_CHECKER_COMMENT", "w");
    CHECKER.inp = inp;
    return inp;
}

Uninhabited checker_finish(CheckOutcome outcome) {
    FILE* proto_file = CHECKER.out_file;
    fprintf(proto_file, "outcome: ");
    switch (outcome) {
        case CheckOutcome::WRONG_ANSWER:
            fprintf(proto_file, "WrongAnswer");
            break;
        case CheckOutcome::CHECKER_LOGIC_ERROR:
            fprintf(proto_file, "CheckerLogicError");
            break;
        case CheckOutcome::OK:
            fprintf(proto_file, "Ok");
            break;
        case CheckOutcome::PRESENTATION_ERROR:
            fprintf(proto_file, "PresentationError");
            break;
    }

    exit(0);
}

const int COMMENT_OUT_BUF_LEN = 4096;

char COMMENT_OUT_BUF[COMMENT_OUT_BUF_LEN];

void check_utils::comment(const char* format, ...) {
    va_list args;
    va_start(args, format);
    int num_written = vsnprintf(COMMENT_OUT_BUF, COMMENT_OUT_BUF_LEN, format, args);
    FILE* f = CHECKER.comment_file;
    fprintf(f, "%s", COMMENT_OUT_BUF);
    if (num_written == COMMENT_OUT_BUF_LEN) {
        fprintf(f, "... (comment was truncated)");
    }
    fprintf(f, "\n");

}

void check_utils::corr_scanf(const char* format, ...) {
    va_list args;
    va_start(args, format);
    int res = vfscanf(CHECKER.inp.corr_answer, format, args);
    va_end(args);
    if (res == EOF) {
        check_utils::comment("fatal: unexpected EOF when reading correct answer");
        checker_finish(CheckOutcome::CHECKER_LOGIC_ERROR);
    }
}

void check_utils::sol_scanf(const char* format, ...) {
    va_list args;
    va_start(args, format);
    int res = vfscanf(CHECKER.inp.sol_answer, format, args);
    va_end(args);
    if (res == EOF) {
        check_utils::comment("error: unexpected EOF when reading provided answer");
        checker_finish(CheckOutcome::PRESENTATION_ERROR);
    }
}

void check_utils::test_scanf(const char* format, ...) {
    va_list args;
    va_start(args, format);
    int res = vfscanf(CHECKER.inp.test, format, args);
    va_end(args);
    if (res == EOF) {
        check_utils::comment("fatal: unexpected EOF when reading test file");
        checker_finish(CheckOutcome::CHECKER_LOGIC_ERROR);
    }
}

void check_utils::check_corr_eof() {
    if (!is_file_eof(CHECKER.inp.corr_answer)) {
        check_utils::comment("fatal: correct answer has data yet");
        checker_finish(CheckOutcome::CHECKER_LOGIC_ERROR);
    }
}

void check_utils::check_test_eof() {
    if (!is_file_eof(CHECKER.inp.test)) {
        check_utils::comment("fatal: test file has data yet");
        checker_finish(CheckOutcome::CHECKER_LOGIC_ERROR);
    }
}

void check_utils::check_sol_eof() {
    if (!is_file_eof(CHECKER.inp.sol_answer)) {
        check_utils::comment("error: solution has data yet");
        checker_finish(CheckOutcome::PRESENTATION_ERROR);
    }
}

bool is_char_whitespace(char c) {
    return isspace(c);
}

bool is_file_eof(FILE* f) {
    const int BUF_SIZE = 256;
    char buf[BUF_SIZE];
    while (true) {
        int nread = fread(buf, 1, BUF_SIZE, f);
        for (int i = 0; i < nread; ++i) {
            if (!is_char_whitespace(buf[i])) {
                return false;
            }
        }
        if (nread == 0) break;
    }
    return true;
}

char* check_utils::next_token(FILE* f) {
    int cap = 16;
    char* out = (char*) malloc(16);
    assert(out);
    int len = 0;
    bool had_data = false;
    while (true) {
        char ch;
        int ret = fread(&ch, 1, 1, f);
        if (ret == -1) {
            check_utils::comment("check_utils: read failed");
            exit(1);
        }
        if (ret == 0) {
            break;
        }
        if (isspace(ch)) {
            if (had_data) {
                break;
            } else {
                continue;
            }
        }
        if (len+1 == cap) {
            cap = 2 * cap;
            out = (char*) realloc(out, cap);
            assert(out);
        }
        had_data = true;
        out[len] = ch;
        ++len;
    }
    if (!had_data) {
        // no chars were read
        free(out);
        return nullptr;
    }
    out[len] = '\0';
    return out;
}