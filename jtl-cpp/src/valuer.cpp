#include <cstdio>
#include <cassert>
#include <cstdarg>
#include <cstdlib>
#include <cstring>
#include "valuer.h"
#include "proto.h"
#include "util.h"


static bool should_run;

void valuer::ValuerSession::select_next_test(valuer::TestId next_test, bool live) {
    assert(1 <= next_test && next_test <= problem_test_count);
    printf("RUN %u %u\n", next_test, live ? 1 : 0);
    fflush(stdout);
}

void valuer::ValuerSession::finish(int score, bool treat_as_full, const JudgeLog& judge_log) {
    printf("DONE %d %d\n", score, (int) treat_as_full);
    printf("%zu\n", judge_log.tests.size());
    char format_buf[STATUS_KIND_MAX_LEN];
    for (const JudgeLogTestEntry& entry : judge_log.tests) {
        StatusKindOps::to_string(entry.status_kind, format_buf);
        printf("%u %s %s %u\n", entry.test_id, format_buf, entry.status_code.c_str(),
               entry.components.flags);
    }
    printf("%zu\n", judge_log.subtasks.size());
    for (const JudgeLogSubtaskEntry& entry : judge_log.subtasks) {
        printf("%u %u %u\n", entry.subtask_id, entry.score, entry.components.flags);
    }
    fflush(stdout);
    should_run = false;
}

void valuer::ValuerSession::run_valuer(valuer::ValuerCallbacks callbacks, void* user_data) {
    assert(callbacks.begin != nullptr);
    assert(callbacks.on_test_end != nullptr);
    assert(jtl::check_pointer((void*) callbacks.on_test_end));
    assert(jtl::check_pointer((void*) callbacks.begin));
    assert(jtl::check_pointer((void*) callbacks.init));

    should_run = true;
    ValuerSession sess;
    sess.data = user_data;
    sess.pub_comments_file = get_env_file("JJS_VALUER_COMMENT_PUB", "w");
    sess.priv_comments_file = get_env_file("JJS_VALUER_COMMENT_PRIV", "w");
    if (callbacks.init) {
        callbacks.init(&sess);
    }
    if (scanf("%u", &sess.problem_test_count) != 1) {
        die("failed to read test count");
    }
    callbacks.begin(&sess);
    while (should_run) {
        int test_id;
        char* status_kind;
        char* status_code;
        if (scanf("%d %ms %ms", &test_id, &status_kind, &status_code) != 3) {
            die("failed to read information about next finished test");
        }
        StatusKind kind = StatusKindOps::parse(status_kind);
        JudgeLogTestEntry test_entry;
        test_entry.status_code = std::string(status_code);
        test_entry.status_kind = kind;
        test_entry.test_id = test_id;
        callbacks.on_test_end(&sess, test_entry);
    }
}

void valuer::ValuerSession::comment_public(const char* format, ...) {
    va_list args;
    va_start(args, format);
    vfprintf(pub_comments_file, format, args);
    va_end(args);
}

void valuer::ValuerSession::comment_private(const char* format, ...) {
    va_list args;
    va_start(args, format);
    vfprintf(priv_comments_file, format, args);
    va_end(args);
}

void* valuer::ValuerSession::get_data() {
    return data;
}

void const* valuer::ValuerSession::get_data() const {
    return data;
}

void valuer::ValuerSession::set_data(void* p) {
    data = p;
}

uint32_t valuer::ValuerSession::get_problem_test_count() {
    return problem_test_count;
}

void valuer::ValuerSession::set_live_score(int score) {
    printf("LIVE-SCORE %d\n", score);
}

void valuer::JudgeLog::add_test_entry(valuer::JudgeLogTestEntry const& test) {
    tests.push_back(test);
}

void valuer::JudgeLog::add_subtask_entry(valuer::JudgeLogSubtaskEntry const& subtask) {
    subtasks.push_back(subtask);
}

valuer::StatusKind valuer::StatusKindOps::parse(const char* s) {
    if (!strcmp(s, "Rejected")) {
        return StatusKind::REJECTED;
    } else if (!strcmp(s, "Accepted")) {
        return StatusKind::ACCEPTED;
    } else if (!strcmp(s, "InternalError")) {
        return StatusKind::INTERNAL_ERROR;
    } else if (!strcmp(s, "Skipper")) {
        return StatusKind::SKIPPED;
    }
    die("in valuer::status_kind_parse: unknown status kind: %s", s);
}

void valuer::StatusKindOps::to_string(const valuer::StatusKind kind, char* buf) {
    switch (kind) {
        case StatusKind::ACCEPTED:
            strcpy(buf, "Accepted");
            break;
        case StatusKind::INTERNAL_ERROR:
            strcpy(buf, "InternalError");
            break;
        case StatusKind::REJECTED:
            strcpy(buf, "Rejected");
            break;
        case StatusKind::SKIPPED:
            strcpy(buf, "Skipped");
            break;
    }
}

bool valuer::StatusKindOps::is_passed(const valuer::StatusKind kind) {
    return kind == StatusKind::ACCEPTED;
}

void valuer::TestVisibleComponents::expose_test_data() {
    flags |= TEST_DATA;
}

void valuer::TestVisibleComponents::expose_output() {
    flags |= OUTPUT;
}

void valuer::TestVisibleComponents::expose_answer() {
    flags |= ANSWER;
}

void valuer::SubtaskVisibleComponents::expose_score() {
    flags |= SCORE;
}
