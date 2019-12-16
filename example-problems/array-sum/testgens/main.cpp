#include <cstdlib>
#include <jjs/jtl.h>

int main() {
    testgen::TestgenSession sess = testgen::init();
    int num_cnt = sess.gen.next_range(1, 10001);
    fprintf(sess.out_file, "%d\n%ld", num_cnt, sess.gen.next_range(1, 10001));
    for (int i = 1; i < num_cnt; i++) {
        fprintf(sess.out_file, " %ld", sess.gen.next_range(1, 10001));
    }
    fprintf(sess.out_file, "\n");
    return 0;
}
