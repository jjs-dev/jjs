//magicbuild:link=jtl
#include <jtl.h>

int main() {
    TestgenInput args = init_testgen();
    fprintf(args.out_file, "%d %d\n", args.test_id, args.test_id * 2 + 1);
}