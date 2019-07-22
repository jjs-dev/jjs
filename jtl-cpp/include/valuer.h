#pragma once

#include "jtl.h"
#include <cstdio>

namespace valuer {

struct ValuerContext {
    int problem_test_count = -1;

    void select_next_test(int next_test);

    void finish(int score, bool treat_as_full);
};

struct ValuerCallbacks {
    void (* init)(ValuerContext* ctx) = nullptr;

    void (* begin)(ValuerContext* ctx) = nullptr;

    void (* on_test_end)(ValuerContext* ctx, int test, bool test_passed, const char* status_code) = nullptr;
};

void run_valuer(ValuerCallbacks callbacks);

void comment_public(const char* format, ...) FORMAT_FN(1);

void comment_private(const char* format, ...) FORMAT_FN(1);
}