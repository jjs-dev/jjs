#pragma once

#include "jtl.h"
#include <cstdio>
#include <cstdint>
#include <string>
#include <vector>

namespace valuer {
const size_t STATUS_KIND_MAX_LEN = 20;
// note: this struct represents only those kinds that make sense in this context
enum class StatusKind {
REJECTED,
ACCEPTED,
INTERNAL_ERROR,
SKIPPED
};

class StatusKindOps {
public:
static StatusKind parse(const char* s);

static void to_string(StatusKind kind, char buf[STATUS_KIND_MAX_LEN]);

static bool is_passed(StatusKind kind);
};

struct VisibleComponents {
    static const uint32_t TEST_DATA = 1;
    static const uint32_t OUTPUT = 2;


    uint32_t flags = 0;

    void expose_test_data();

    void expose_output();
};

using TestId = uint32_t;

struct JudgeLogEntry {
    TestId test_id;
    std::string status_code;
    StatusKind status_kind;
    uint32_t score;
    VisibleComponents components;
};

struct JudgeLog {
    std::string name;
    std::vector<JudgeLogEntry> entries;
};

struct ValuerContext {
    int problem_test_count = -1;

    void select_next_test(TestId next_test);

    void finish(int score, bool treat_as_full, const JudgeLog& judge_log);
};

struct ValuerCallbacks {
    void (* init)(ValuerContext* ctx) = nullptr;

    void (* begin)(ValuerContext* ctx) = nullptr;

    void (* on_test_end)(ValuerContext* ctx, TestId test, StatusKind status_kind, const char* status_code) = nullptr;
};

void run_valuer(ValuerCallbacks callbacks);

void comment_public(const char* format, ...) FORMAT_FN(1);

void comment_private(const char* format, ...) FORMAT_FN(1);
}