#include "jtl.h"
#include "proto.h"
#include <cstdlib>
#include <cstdio>
#include <cstdarg>
#include <cctype>
#include <cassert>

struct CheckerData {
    checker::CheckerInput inp;
    FILE* out_file;
    FILE* comment_file;
};

CheckerData CHECKER;

checker::CheckerInput checker::init() {
    checker::CheckerInput inp;
    inp.corr_answer = get_env_file("JJS_CORR", "r");
    inp.sol_answer = get_env_file("JJS_SOL", "r");
    inp.test = get_env_file("JJS_TEST", "r");
    CHECKER.out_file = get_env_file("JJS_CHECKER_OUT", "w");
    CHECKER.comment_file = get_env_file("JJS_CHECKER_COMMENT", "w");
    CHECKER.inp = inp;
    return inp;
}

Uninhabited checker::finish(Outcome outcome) {
    FILE* proto_file = CHECKER.out_file;
    fprintf(proto_file, "outcome: ");
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
    int num_written = vsnprintf(COMMENT_OUT_BUF, COMMENT_OUT_BUF_LEN, format, args);
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

bool is_char_whitespace(char c) {
    return c == ' ' || c == '\n' || c == '\t';
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

/// returns owning pointer to token. This pointer should be freed by free()
char* checker::next_token(FILE* f) {
    int cap = 16;
    char* out = (char*) malloc(16);
    assert(out);
    int len = 0;
    bool had_data = false;
    while (true) {
        char ch;
        int ret = fread(&ch, 1, 1, f);
        if (ret == -1) {
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