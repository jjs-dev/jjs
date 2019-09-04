#include "valuer.h"
#include <cassert>
#include <cstdio>
#include <cstring>
#include <unistd.h>
#include <inipp.h>
#include <fstream>

using namespace valuer;

static JudgeLog judge_log;

struct Params {
    int open_test_count = 1;
};

using Ini = inipp::Ini<char>;

static Params read_config() {
    Params p {};
    Ini ini;
    std::ifstream cfg;
    cfg.open("./cfg.ini");
    if (cfg.fail()) {
        char const* const err_buf = strerror(errno);
        comment_private("warning: failed open config file: %s\n", err_buf);
        comment_private("note: will use defaults\n");
        return p;
    }
    ini.parse(cfg);
    auto const& main_sec = ini.sections[""];
    if (main_sec.count("open-test-count")) {
        inipp::extract(main_sec.at("open-test-count"), p.open_test_count);
    }

    return p;
}

void init(ValuerContext* const ctx) {
    auto const cfg = read_config();
    auto* const params  = new Params;
    *params = cfg;
    ctx->data = params;
}

Params const& get_params(ValuerContext const* const ctx) {
    return *(Params*) ctx->data;
}

void begin(ValuerContext* const ctx) {
    assert(ctx->problem_test_count >= 1);
    ctx->select_next_test(1);
}

void on_test_end(ValuerContext* ctx, TestId test, StatusKind status_kind, const char* status_code) {
    JudgeLogEntry entry;
    entry.status_kind = status_kind;
    entry.status_code = status_code;
    entry.test_id = test;
    entry.score = 0;
    if (test <= get_params(ctx).open_test_count) {
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
    cbs.init = init;
    cbs.on_test_end = on_test_end;
    cbs.begin = begin;
    judge_log.name = "main";
    run_valuer(cbs);
    return 0;
}