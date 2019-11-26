#pragma once

#include "jtl.h"
#include <cstdint>
#include <cstdio>
#include <string>
#include <vector>

namespace valuer {
const size_t STATUS_KIND_MAX_LEN = 20;
// note: this struct represents only those kinds that make sense in this context
enum class StatusKind { REJECTED, ACCEPTED, INTERNAL_ERROR, SKIPPED };

class StatusKindOps {
  public:
    static StatusKind parse(const char* s);

    static void to_string(StatusKind kind, char buf[STATUS_KIND_MAX_LEN]);

    static bool is_passed(StatusKind kind);
};

struct TestVisibleComponents {
    static const uint32_t TEST_DATA = 1;
    static const uint32_t OUTPUT = 2;
    static const uint32_t ANSWER = 4;

    uint32_t flags = 0;

    void expose_test_data();

    void expose_output();

    void expose_answer();
};

using TestId = uint32_t;

struct JudgeLogTestEntry {
    TestId test_id;
    std::string status_code;
    StatusKind status_kind;
    TestVisibleComponents components;
};

using SubtaskId = uint32_t;

struct SubtaskVisibleComponents {
    static const uint32_t SCORE = 1;

    uint32_t flags;

    void expose_score();
};

struct JudgeLogSubtaskEntry {
    SubtaskId subtask_id;
    uint32_t score;
    SubtaskVisibleComponents components;
};

struct JudgeLog {
    std::string name;
    std::vector<JudgeLogTestEntry> tests;
    std::vector<JudgeLogSubtaskEntry> subtasks;

    void add_test_entry(JudgeLogTestEntry const& test);

    void add_subtask_entry(JudgeLogSubtaskEntry const& entry);
};

class ValuerSession;

struct ValuerCallbacks {
    void (*init)(ValuerSession* sess) = nullptr;

    void (*begin)(ValuerSession* sess) = nullptr;

    void (*on_test_end)(ValuerSession* sess,
                        JudgeLogTestEntry test_info) = nullptr;
};

class ValuerSession {
    void* data = nullptr;

    uint32_t problem_test_count = -1;

    JudgeLog log;
    FILE* pub_comments_file = nullptr;
    FILE* priv_comments_file = nullptr;

  public:
    void* get_data();

    [[nodiscard]] void const* get_data() const;

    void set_data(void* data);

    uint32_t get_problem_test_count();

    void select_next_test(TestId next_test, bool live);

    void set_live_score(int live_score);

    void finish(int score, bool treat_as_full, const JudgeLog& judge_log);

    void comment_public(const char* format, ...) PRINT_FORMAT_FN(2);

    void comment_private(const char* format, ...) PRINT_FORMAT_FN(2);

    static void run_valuer(ValuerCallbacks callbacks,
                           void* user_data = nullptr);
};
} // namespace valuer