#include "testgen.h"
#include "rand-ffi.h"
#include "proto.h"
#include <mutex>
#include <cstdio>
#include <cstring>

testgen::Generator::Generator(uint8_t seed[32]) {
    impl = random_init(seed);
}

testgen::Word testgen::Generator::next_u64() {
    return random_gen64((Random*) impl);
}

testgen::Word testgen::Generator::next_range(testgen::Word lo, testgen::Word hi) {
    return random_gen_range((Random*) impl, lo, hi);
}

testgen::Generator testgen::Generator::clone() {
    auto new_impl = random_clone((Random*) impl);
    return {new_impl};
}

testgen::Generator* GLOBAL_RND;

testgen::Generator* testgen::Generator::open_global() {
    if (!GLOBAL_RND) {
        fprintf(stderr, "fatal error: Generator is requested, but is is not initialized yet.\n");
        exit(1);
    }
    return GLOBAL_RND;
}

void init_global_gen(testgen::Seed seed) {
    if (GLOBAL_RND) {
        fprintf(stderr, "fatal error: Global generator is constructed twice");
    }
    GLOBAL_RND = new testgen::Generator(seed);
}

testgen::Input testgen::init() {
    testgen::Input ti;
    ti.test_id = get_env_int("JJS_TEST_ID");
    ti.out_file = get_env_file("JJS_TEST", "w");
    auto rand_seed = get_env_hex("JJS_RANDOM_SEED");
    if (rand_seed.len != 32) {
        fprintf(stderr, "rand_seed has incorrect length (%zu instead of 32)\n", rand_seed.len);
        exit(1);
    }
    memcpy(ti.random_seed, rand_seed.head, 32);
    init_global_gen(ti.random_seed);
    rand_seed.dealloc();
    //testgen::in
    return ti;
}
