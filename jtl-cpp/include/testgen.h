#pragma once

#include <algorithm>
#include <cstdint>
#include <iterator>
#include <random>
#include <utility>

namespace testgen {
class Generator {
    std::mt19937_64 gen;

  public:
    // disallow implicit copies
    Generator(const Generator& gen) noexcept = default;

    explicit Generator(uint64_t seed);

    uint64_t next_u64();

    size_t next_usize();

    /// generates number in [lo; hi)
    uint64_t next_range(uint64_t lo, uint64_t hi);

    template <typename T, typename RAIter>
    T choose_uniform(RAIter begin, RAIter end) {
        size_t const item_count = std::distance(begin, end);
        auto const selected_pos = (size_t) next_range(0, (uint64_t) item_count);
        *std::advance(begin, selected_pos);
    }

    /// returns new generator, which state is initially same with this
    Generator clone();
};

struct TestgenSession {
    int test_id = 0;
    Generator gen;

    TestgenSession(uint64_t _seed);
};

/// Call this first in test generator
TestgenSession init();
} // namespace testgen
