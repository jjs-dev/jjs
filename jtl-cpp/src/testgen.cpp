#include "testgen.h"
#include "jtl.h"
#include "proto.h"
#include <cassert>
#include <cstdio>
#include <cstring>
#include <mutex>

testgen::Generator::Generator(uint64_t seed) : gen(seed) {}

uint64_t testgen::Generator::next_u64() { return gen(); }

/// Returns random number in [0; n)
static uint64_t get_rand(uint64_t n, std::mt19937_64& gen) {
    assert(n != 0);
    uint64_t bits = n;
    // step one: we want `bits` to contain highest bit of n, and all smaller
    bits |= (bits >> 1u);
    bits |= (bits >> 2u);
    bits |= (bits >> 4u);
    bits |= (bits >> 8u);
    bits |= (bits >> 16u);
    bits |= (bits >> 32u);
    while (true) {
        uint64_t s = gen();
        s &= bits;
        // why is it fast: bits is smaller than 2*n, so probability that this
        // iteration succeed is at least 0.5
        if (s < n) {
            return s;
        }
    }
}

uint64_t testgen::Generator::next_range(uint64_t lo, uint64_t hi) {
    assert(lo < hi);
    return lo + get_rand(hi - lo, gen);
}
testgen::TestgenSession::TestgenSession(uint64_t _seed) : gen(_seed) {}
testgen::TestgenSession testgen::init() {
    auto rand_seed = get_env_hex("JJS_RANDOM_SEED");
    if (rand_seed.len != 8) {
        die("rand_seed has incorrect length (%zu instead of 8)\n",
            rand_seed.len);
    }
    uint64_t random_seed;
    memcpy(&random_seed, rand_seed.head.get(), 8);
    testgen::TestgenSession sess {random_seed};
    sess.test_id = get_env_int("JJS_TEST_ID");
    return sess;
}
