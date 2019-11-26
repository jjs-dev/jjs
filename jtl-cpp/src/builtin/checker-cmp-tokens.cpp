#include <checker.h>
#include <cstddef>
#include <cstdio>
#include <cstring>

using namespace checker;
struct Random;

struct Args {
    bool enable_epsilon = false;
    long double epsilon = 0.0;
    bool ignore_case = false;
};

bool is_float(const char* s) {
    const size_t n = strlen(s);
    size_t cnt = 0;
    for (size_t i = 0; i < n; ++i) {
        if (s[i] != '.' && (s[i] < '0' || s[i] > '9')) {
            return false;
        }
        if (s[i] == '.') {
            ++cnt;
        }
    }
    return cnt <= 1 && s[0] != '.' && s[n - 1] != '.';
}

bool compare_tokens(char* expected, char* actual, const Args& args) {
    if (is_float(expected) && args.enable_epsilon) {
        long double exp = strtold(expected, nullptr);
        long double act = strtold(actual, nullptr);
        return compare_epsilon(exp, act, args.epsilon);
    } else if (args.ignore_case) {
        return compare_strings_ignore_case(expected, actual);
    } else {
        return strcmp(expected, actual) == 0;
    }
}

int main(int argc, char** argv) {
    checker::CheckerInput checker_input = init();
    Args args;
    for (size_t i = 1; i < argc; ++i) {
        if (strcmp(argv[i], "--epsilon") == 0) {
            if (i + 1 == argc) {
                fprintf(stderr, "Error: --epsilon was not given value");
                finish(Outcome::CHECKER_LOGIC_ERROR);
            }
            char* endptr;
            long double eps = strtold(argv[i + 1], &endptr);
            if (endptr == argv[i + 1]) {
                fprintf(stderr, "Error: %s is not valid long double value",
                        argv[i + 1]);
                finish(Outcome::CHECKER_LOGIC_ERROR);
            }
            args.enable_epsilon = true;
            args.epsilon = eps;
        } else if (strcmp(argv[i], "--ignore-case") == 0) {
            args.ignore_case = true;
        }
    }
    size_t i = 0;
    while (true) {
        char* actual = next_token(checker_input.sol_answer);
        char* expected = next_token(checker_input.corr_answer);
        if (!expected && actual) {
            comment("error: early EOF in actual answer on position %zu", i);
            comment("note: next expected token was %s", expected);
            finish(Outcome::WRONG_ANSWER);
        }
        if (expected && !actual) {
            comment("error: actual answer contains additional tokens, starting "
                    "from %zu",
                    i);
            comment("note: next actual token was %s", actual);
            finish(Outcome::WRONG_ANSWER);
        }
        if (!expected) {
            break;
        }
        bool eq = compare_tokens(expected, actual, args);
        if (!eq) {
            comment("error: token mismatch on position %zu", i);
            comment("note: expected %s, got %s", expected, actual);

            finish(Outcome::WRONG_ANSWER);
        }
        ++i;
    }

    comment("success: %zu tokens", i);
    finish(Outcome::OK);
}