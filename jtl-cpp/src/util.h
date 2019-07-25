#pragma once

#include <cstdio>

bool check_pointer(void* ptr);

struct FileInfo {
    char* path;
    FILE* file;
};