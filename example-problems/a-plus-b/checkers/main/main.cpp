//magicbuild:link=jtl
#include <jtl.h>



int main() {
    CheckerInput inp = init_checker();
    int contestant_answer, correct_answer;
    check_utils::corr_scanf("%d", &correct_answer);
    check_utils::sol_scanf("%d", &contestant_answer);
    check_utils::check_corr_eof();
    check_utils::check_sol_eof();
    if (contestant_answer == correct_answer) {
        checker_finish(CheckOutcome::OK);
    } else {
        checker_finish(CheckOutcome::WRONG_ANSWER);
    }
}