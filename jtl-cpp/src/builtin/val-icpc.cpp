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

static Params read_config(ValuerSession* sess) {
    Params p {};
    Ini ini;
    std::ifstream cfg;
    cfg.open("./cfg.ini");
    if (cfg.fail()) {
        char const* const err_buf = strerror(errno);
        sess->comment_private("warning: failed open config file: %s\n", err_buf);
        sess->comment_private("note: will use defaults\n");
        return p;
    }
    ini.parse(cfg);
    auto const& main_sec = ini.sections[""];
    if (main_sec.count("open-test-count")) {
        inipp::extract(main_sec.at("open-test-count"), p.open_test_count);
    }

    return p;
}

void init(ValuerSession* const sess) {
    auto const cfg = read_config(sess);
    auto* const params = new Params;
    *params = cfg;
    sess->set_data(params);
}

Params const& get_params(ValuerSession const* const sess) {
    return *(Params*) sess->get_data();
}

void begin(ValuerSession* const sess) {
    assert(sess->get_problem_test_count() >= 1);
    sess->select_next_test(1, true);
}

void on_test_end(ValuerSession* sess, JudgeLogTestEntry finished_test) {
    bool next_test_is_sample = (finished_test.test_id + 1) <= get_params(sess).open_test_count;
    if (finished_test.test_id <= get_params(sess).open_test_count) {
        finished_test.components.expose_output();
        finished_test.components.expose_test_data();
        finished_test.components.expose_answer();
    }
    judge_log.add_test_entry(finished_test);

    const bool test_passed = StatusKindOps::is_passed(finished_test.status_kind);
    const bool should_stop = !test_passed || (finished_test.test_id == sess->get_problem_test_count());
    if (should_stop) {
        if (test_passed) {
            sess->finish(100, true, judge_log);
            sess->comment_public("ok, all tests passed");
        } else {
            sess->finish(0, false, judge_log);
            sess->comment_public("solution failed on test %d: (status %s)", finished_test.test_id,
                                 finished_test.status_code.c_str());
        }
    } else {
        sess->select_next_test(finished_test.test_id + 1, true);
        if (next_test_is_sample) {
            sess->set_live_score(50);
        }
    }
}


int main() {
    ValuerCallbacks cbs;
    cbs.init = init;
    cbs.on_test_end = on_test_end;
    cbs.begin = begin;
    judge_log.name = "main";
    ValuerSession::run_valuer(cbs);
    return 0;
}