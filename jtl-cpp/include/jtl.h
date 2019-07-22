#pragma once

#include <cstdio>
#include "testgen.h"

#ifdef __GNUC__
#define FORMAT_FN(x) __attribute__ (( format( printf, x, x+1 ) ))
#else
#define FORMAT_FN(x)
#endif

/// return type for functions that do not return
enum class Uninhabited {
};

/// utility functions checks than only whitespace chars are remaining in file
bool is_file_eof(FILE* f);