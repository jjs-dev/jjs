#pragma once

#include <cstdio>
namespace jtl {
bool check_pointer(void* ptr);

struct FileInfo {
    char* path;
    FILE* file;
};
} // namespace jtl