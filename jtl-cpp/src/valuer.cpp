#include <cstdio>
#include <cassert>
#include <cstdarg>
#include <cstdlib>
#include <cstring>
#include "valuer.h"
#include "proto.h"
#include "unistd.h"
#include "util.h"

FILE* pub_comments_file;
FILE* priv_comments_file;

static bool should_run;

void valuer::ValuerContext::select_next_test(int next_test) {
    assert(1 <= next_test && next_test <= problem_test_count);
    printf("RUN %d\n", next_test);
    fflush(stdout);
}

void valuer::ValuerContext::finish(int score, bool treat_as_full, const JudgeLog& judge_log) {
    printf("DONE %d %d %zu\n", score, (int) treat_as_full, judge_log.entries.size());
    char format_buf[STATUS_KIND_MAX_LEN];
    for (const JudgeLogEntry& entry: judge_log.entries) {
        StatusKindOps::to_string(entry.status_kind, format_buf);
        printf("%d %s %s %d\n", entry.test_id, format_buf, entry.status_code.c_str(), entry.score);
    }
    fflush(stdout);
    should_run = false;
}

void valuer::run_valuer(valuer::ValuerCallbacks callbacks) {
    assert(callbacks.begin != nullptr);
    assert(callbacks.on_test_end != nullptr);
    assert(check_pointer((void*) callbacks.on_test_end));
    assert(check_pointer((void*) callbacks.begin));
    assert(check_pointer((void*) callbacks.init));

    should_run = true;
    ValuerContext ctx;
    pub_comments_file = get_env_file("JJS_VALUER_COMMENT_PUB", "w");
    priv_comments_file = get_env_file("JJS_VALUER_COMMENT_PRIV", "w");
    if (callbacks.init) {
        callbacks.init(&ctx);
    }
    scanf("%d", &ctx.problem_test_count);
    callbacks.begin(&ctx);
    while (should_run) {
        int test_id;
        char* status_kind;
        char* status_code;
        scanf("%d %ms %ms", &test_id, &status_kind, &status_code);
        StatusKind kind = StatusKindOps::parse(status_kind);
        callbacks.on_test_end(&ctx, test_id, kind, status_code);
    }
}

void valuer::comment_public(const char* format, ...) {
    va_list args;
    va_start(args, format);
    vfprintf(pub_comments_file, format, args);
    va_end(args);
}

void valuer::comment_private(const char* format, ...) {
    va_list args;
    va_start(args, format);
    vfprintf(priv_comments_file, format, args);
    va_end(args);
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
    comment_private("in valuer::status_kind_parse: unknown status kind: %s", s);
    exit(1);
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
