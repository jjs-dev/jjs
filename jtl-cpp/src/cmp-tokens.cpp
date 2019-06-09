#include <jtl.h>
int main() {
    CheckerInput checker_input = init_checker();
    int i = 0;
    while (true) {
        char* actual = check_utils::next_token(checker_input.sol_answer);
        char* expected = check_utils::next_token(checker_input.corr_answer);
        if (!expected && actual) {
            check_utils::comment("error: early EOF in actual answer on position %d", i);
            check_utils::comment("note: next expected token was %s", expected);
            checker_finish(CheckOutcome::WRONG_ANSWER);
        }
        if (expected && !actual) {
            check_utils::comment("error: actual answers contains additional tokens, starting from %d", i);
            check_utils::comment("note: next actual token was %s", actual);
            checker_finish(CheckOutcome::WRONG_ANSWER);
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
            check_utils::comment("error: token mismatch on position %d", i);
            check_utils::comment("note: expected %s, got %s", expected, actual);

            checker_finish(CheckOutcome::WRONG_ANSWER);
        }
        ++i;
    }
    check_utils::comment("success: %d tokens", i);
    checker_finish(CheckOutcome::OK);

}