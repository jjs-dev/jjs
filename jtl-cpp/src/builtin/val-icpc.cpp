#include "valuer.h"
#include <cassert>
#include <cstdio>
#include <cstring>
#include <unistd.h>

using namespace valuer;

static void foo(const char* msg) {
    write(-1, msg, strlen(msg));
}

void begin(ValuerContext* ctx) {
    assert(ctx->problem_test_count >= 1);
    ctx->select_next_test(1);
    foo("ICPCValuer: selected test");
}

void on_test_end(ValuerContext* ctx, int test, bool test_passed, const char* status_code) {
    bool should_stop = !test_passed || (test == ctx->problem_test_count);
    if (should_stop) {
        if (test_passed) {
            ctx->finish(100, true);
            comment_public("ok, all tests passed");
        } else {
            ctx->finish(0, false);
            comment_private("solution failed on test %d: (raw status %s)", test, status_code);
        }
    } else {
        ctx->select_next_test(test + 1);
    }
}


int main() {
    ValuerCallbacks cbs;
    cbs.on_test_end = on_test_end;
    cbs.begin = begin;
    run_valuer(cbs);
}