#pragma once

#include <cstdio>
#include "testgen.h"


/// return type for functions that do not return
enum class Uninhabited {
};

/// utility functions checks than only whitespace chars are remaining in file
bool is_file_eof(FILE* f);