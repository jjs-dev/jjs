#include <cstdlib>
#include <jjs/jtl.h>

int main() {
    testgen::TestgenSession sess = testgen::init();
    int num_cnt = sess.gen.next_range(1, 10001);
    printf("%d\n%ld", num_cnt, sess.gen.next_range(1, 10001));
    for (int i = 1; i < num_cnt; i++) {
        printf(" %ld", sess.gen.next_range(1, 10001));
    }
    printf("\n");
    return 0;
}
