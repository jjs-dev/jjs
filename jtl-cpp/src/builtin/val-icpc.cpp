#include "valuer.h"
#include <cassert>
#include <cstdio>
#include <cstring>
#include <unistd.h>

using namespace valuer;

static JudgeLog judge_log;

void begin(ValuerContext* ctx) {
    assert(ctx->problem_test_count >= 1);
    ctx->select_next_test(1);
}

void on_test_end(ValuerContext* ctx, TestId test, StatusKind status_kind, const char* status_code) {
    JudgeLogEntry entry;
    entry.status_kind = status_kind;
    entry.status_code = status_code;
    entry.test_id = test;
    entry.score = 0;
    if (test == 1) {
        entry.components.expose_output();
        entry.components.expose_test_data();
        entry.components.expose_answer();
    }
    judge_log.entries.push_back(entry);
    const bool test_passed = StatusKindOps::is_passed(status_kind);
    const bool should_stop = !test_passed || (test == ctx->problem_test_count);
    if (should_stop) {
        if (test_passed) {
            ctx->finish(100, true, judge_log);
            comment_public("ok, all tests passed");
        } else {
            ctx->finish(0, false, judge_log);
            comment_public("solution failed on test %d: (status %s)", test, status_code);
        }
    } else {
        ctx->select_next_test(test + 1);
    }
}


int main() {
    ValuerCallbacks cbs;
    cbs.on_test_end = on_test_end;
    cbs.begin = begin;
    judge_log.name = "main";
    run_valuer(cbs);
    return 0;
}