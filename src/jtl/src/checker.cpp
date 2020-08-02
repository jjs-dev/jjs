#include "checker.h"
#include "jtl.h"
#include "proto.h"
#include <cassert>
#include <cmath>
#include <cstdarg>

struct CheckerData {
    checker::CheckerInput inp;
    FILE* out_file = nullptr;
    FILE* comment_file = nullptr;
};

CheckerData CHECKER;

checker::CheckerInput checker::init(bool open_files) {
    checker::CheckerInput inp;
    if (open_files) {
        inp.corr_answer = get_env_file("JJS_CORR", "r");
        inp.sol_answer = get_env_file("JJS_SOL", "r");
        inp.test = get_env_file("JJS_TEST", "r");
    } else {
        inp.fd_corr = get_env_int("JJS_CORR");
        inp.fd_sol = get_env_int("JJS_SOL");
        inp.fd_test = get_env_int("JJS_TEST");
    }
    CHECKER.out_file = get_env_file("JJS_CHECKER_OUT", "w");
    CHECKER.comment_file = get_env_file("JJS_CHECKER_COMMENT", "w");
    CHECKER.inp = inp;
    return inp;
}

void checker::finish(Outcome outcome) {
    FILE* proto_file = CHECKER.out_file;
    fprintf(proto_file, "outcome=");
    switch (outcome) {
    case Outcome::WRONG_ANSWER:
        fprintf(proto_file, "WrongAnswer");
        break;
    case Outcome::CHECKER_LOGIC_ERROR:
        fprintf(proto_file, "CheckerLogicError");
        break;
    case Outcome::OK:
        fprintf(proto_file, "Ok");
        break;
    case Outcome::PRESENTATION_ERROR:
        fprintf(proto_file, "PresentationError");
        break;
    }

    exit(0);
}

const int COMMENT_OUT_BUF_LEN = 4096;

char COMMENT_OUT_BUF[COMMENT_OUT_BUF_LEN];

void checker::comment(const char* format, ...) {
    va_list args;
    va_start(args, format);
    int num_written =
        vsnprintf(COMMENT_OUT_BUF, COMMENT_OUT_BUF_LEN, format, args);
    va_end(args);
    FILE* f = CHECKER.comment_file;
    fprintf(f, "%s", COMMENT_OUT_BUF);
    if (num_written == COMMENT_OUT_BUF_LEN) {
        fprintf(f, "... (comment was truncated)");
    }
    fprintf(f, "\n");
}

void checker::corr_scanf(const char* format, ...) {
    va_list args;
    va_start(args, format);
    int res = vfscanf(CHECKER.inp.corr_answer, format, args);
    va_end(args);
    if (res == EOF) {
        comment("fatal: unexpected EOF when reading correct answer");
        finish(Outcome::CHECKER_LOGIC_ERROR);
    }
}

void checker::sol_scanf(const char* format, ...) {
    va_list args;
    va_start(args, format);
    int res = vfscanf(CHECKER.inp.sol_answer, format, args);
    va_end(args);
    if (res == EOF) {
        comment("error: unexpected EOF when reading provided answer");
        finish(Outcome::PRESENTATION_ERROR);
    }
}

void checker::test_scanf(const char* format, ...) {
    va_list args;
    va_start(args, format);
    int res = vfscanf(CHECKER.inp.test, format, args);
    va_end(args);
    if (res == EOF) {
        comment("fatal: unexpected EOF when reading test file");
        finish(Outcome::CHECKER_LOGIC_ERROR);
    }
}

void checker::check_corr_eof() {
    if (!is_file_eof(CHECKER.inp.corr_answer)) {
        comment("fatal: correct answer has data yet");
        finish(Outcome::CHECKER_LOGIC_ERROR);
    }
}

void checker::check_test_eof() {
    if (!is_file_eof(CHECKER.inp.test)) {
        comment("fatal: test file has data yet");
        finish(Outcome::CHECKER_LOGIC_ERROR);
    }
}

void checker::check_sol_eof() {
    if (!is_file_eof(CHECKER.inp.sol_answer)) {
        comment("error: solution has data yet");
        finish(Outcome::PRESENTATION_ERROR);
    }
}

char* checker::next_token(FILE* f) {
    int cap = 16;
    char* out = (char*) check_oom(malloc(16));
    int len = 0;
    bool had_data = false;
    while (true) {
        char ch;
        clearerr(f);
        int ret = fread(&ch, 1, 1, f);
        if (ret == 0 && ferror(f) != 0) {
            comment("check_utils: read failed");
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
        if (len + 1 == cap) {
            cap = 2 * cap;
            void* const realloced = realloc(out, cap);
            out = (char*) check_oom(realloced);
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

bool checker::compare_epsilon(long double expected, long double actual,
                              long double epsilon) {
    assert(std::isfinite(expected));
    if (!std::isfinite(actual)) {
        return false;
    }

    long double absolute_error = std::abs(expected - actual);
    if (std::abs(expected) < 1.0) {
        return absolute_error <= epsilon;
    } else {
        long double relative_error = absolute_error / std::abs(expected);
        return relative_error <= epsilon;
    }
}

bool checker::compare_strings_ignore_case(const char* lhs, const char* rhs) {
    while (true) {
        char a = *lhs;
        char b = *rhs;
        if (a == '\0' || b == '\0') {
            return a == b;
        }
        a = tolower(a);
        b = tolower(b);
        if (a != b) {
            return false;
        }
        ++lhs;
        ++rhs;
    }
}
