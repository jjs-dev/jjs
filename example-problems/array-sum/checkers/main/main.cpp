//magicbuild:link=jtl
#include <jtl.h>

using namespace check_utils;

int main()
{
    CheckerInput args = init_checker();
    int n;
    test_scanf("%d", &n);
    long long ans = 0;
    for(int i = 0; i < n; i++)
    {
        int cur;
        test_scanf("%d", &cur);
        ans += cur;
    }
    long long sol_ans;
    sol_scanf("%lld", &sol_ans);
    check_test_eof();
    check_sol_eof();
    checker_finish((sol_ans==ans)?CheckOutcome::OK:CheckOutcome::WRONG_ANSWER);
    return 0;
}
