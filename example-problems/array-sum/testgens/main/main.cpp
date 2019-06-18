//magicbuild:link=jtl
#include <jtl.h>
#include <cstdlib>

int randrange(int a, int b = -1)
{
    if(b == -1)
    {
        b = a;
        a = 0;
    }
    while(b - a > 0)
    {
        int mid = a + (b - a) / 2;
        if(rand() & 1)
            a = mid;
        else
            b = mid;
    }
    return a;
}

int main()
{
    TestgenInput args = init_testgen();
    srand(args.test_id);
    int num_cnt = randrange(1, 10001);
    fprintf(args.out_file, "%d\n%d", num_cnt, randrange(1, 10001));
    for(int i = 1; i < num_cnt; i++)
        fprintf(args.out_file, " %d", randrange(1, 10001));
    fprintf(args.out_file, "\n");
    return 0;
}
