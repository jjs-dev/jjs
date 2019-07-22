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

static void foo(const char* msg) {
    write(-1, msg, strlen(msg));
}

void valuer::ValuerContext::select_next_test(int next_test) {
    assert(1 <= next_test && next_test <= problem_test_count);
    printf("RUN %d\n", next_test);
    foo("enter flush");
    fflush(stdout);
    foo("leave flush");
}

void valuer::ValuerContext::finish(int score, bool treat_as_full) {
    printf("DONE %d %d\n", score, (int) treat_as_full);
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
    foo("begin returned control");
    while (should_run) {
        foo("running regular iteration");
        int test_id;
        int test_passed;
        char status_code[64];
        scanf("%d %d %s", &test_id, &test_passed, status_code);
        callbacks.on_test_end(&ctx, test_id, (bool) test_passed, status_code);
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
