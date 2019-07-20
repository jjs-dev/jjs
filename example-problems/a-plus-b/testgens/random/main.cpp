#include "jjs/jtl.h"
long long NUM = 1e18;

long long gen() {
    testgen::Generator* gen = testgen::Generator::open_global();
    return gen->next_range(-NUM, NUM);
}

int main() {
    auto inp = testgen::init();
    auto a = gen();
    auto b = gen();
    fprintf(inp.out_file, "%lld %lld\n", a, b);
}
