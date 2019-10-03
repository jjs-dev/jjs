#include <jjs/jtl.h>
#include <cstdlib>

int main() {
    testgen::Input args = testgen::init();
    testgen::Generator* gen = testgen::Generator::open_global();
    int num_cnt = gen->next_range(1, 10001);
    fprintf(args.out_file, "%d\n%ld", num_cnt, gen->next_range(1, 10001));
    for (int i = 1; i < num_cnt; i++) {
        fprintf(args.out_file, " %ld", gen->next_range(1, 10001));
    }
    fprintf(args.out_file, "\n");
    return 0;
}
