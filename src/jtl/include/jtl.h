#pragma once

#include "testgen.h"
#include <cstdio>

#ifdef __GNUC__
#define SCAN_FORMAT_FN(x) __attribute__((format(scanf, x, x + 1)))
#define PRINT_FORMAT_FN(x) __attribute__((format(printf, x, x + 1)))
#else
#define SCAN_FORMAT_FN(x)
#define PRINT_FORMAT_FN(x)
#endif

#ifdef __GNUC__
#define ATTR_NORETURN __attribute__((noreturn))
#else
#define ATTR_NORETURN
#endif

#ifdef __GNUC__
#define ATTR_RET_NONNULL __attribute__((returns_nonnull))
#else
#define ATTR_RET_NONNULL
#endif

/// utility functions checks than only whitespace chars are remaining in file
bool is_file_eof(FILE* f);

void oom() ATTR_NORETURN;

void* check_oom(void* ptr) ATTR_RET_NONNULL;

void die(char const* message, ...) ATTR_NORETURN PRINT_FORMAT_FN(1);