#include <jtl.h>

using namespace checker;
struct Random;

int main() {
    checker::CheckerInput checker_input = init();
    int i = 0;
    while (true) {
        char* actual = next_token(checker_input.sol_answer);
        char* expected = next_token(checker_input.corr_answer);
        if (!expected && actual) {
            comment("error: early EOF in actual answer on position %d", i);
            comment("note: next expected token was %s", expected);
            finish(Outcome::WRONG_ANSWER);
        }
        if (expected && !actual) {
            comment("error: actual answers contains additional tokens, starting from %d", i);
            comment("note: next actual token was %s", actual);
            finish(Outcome::WRONG_ANSWER);
        }
        if (!expected && !actual) {
            break;
        }
        bool eq = true;
        for (int j = 0; expected[j] != '\0'; ++j) {
            if (actual[j] != expected[j]) {
                eq = false;
                break;
            }
        }
        if (!eq) {
            comment("error: token mismatch on position %d", i);
            comment("note: expected %s, got %s", expected, actual);

            finish(Outcome::WRONG_ANSWER);
        }
        ++i;
    }

    comment("success: %d tokens", i);
    finish(Outcome::OK);

}