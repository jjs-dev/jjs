#pragma once

#include <cstdint>
#include <iterator>
#include <utility>
#include <algorithm>

namespace testgen {
    using Seed = uint8_t[32];

    using Word = int64_t;

    class Generator {
        void* impl;

        Generator(void* impl) : impl(impl) {}

        Generator(Generator&& oth) {
            std::swap(impl, oth.impl);
        }

    public:
        Generator(const Generator& gen) = delete;

        explicit Generator(Seed seed);

        Word next_u64();

        /// generates number in [lo; hi)
        Word next_range(Word lo, Word hi);

        template<typename T, typename RAIter>
        T choose_uniform(RAIter begin, RAIter end) {
            Word n = (Word) std::distance(begin, end);
            Word k = next_range(0, n);
            *std::advance(begin, (size_t) k);
        }

        /// returns new generator, which state is initially same with this
        Generator clone();

        /// returns pointer to global generator. It is automatically initialized, and should be used in most cases
        static Generator* open_global();
    };

    struct Input {
        FILE* out_file;
        int test_id;
        Seed random_seed;
    };

    /// Call this first in test generator
    Input init();
}
