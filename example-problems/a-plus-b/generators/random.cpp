#include "jjs/jtl.h"
long long NUM = 1e18;

long long gen(testgen::TestgenSession& sess) {
    uint64_t t = sess.gen.next_range(0, 2 * NUM);
    long long l = static_cast<long long>(t);
    return l - NUM;
}

int main() {
    auto sess = testgen::init();
    auto a = gen(sess);
    auto b = gen(sess);
    printf("%lld %lld\n", a, b);
}
