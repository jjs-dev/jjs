#pragma once

#include <cstdint>
#include <cstdio>
#include <memory>

char* get_env(const char* var_name);

int get_env_int(const char* var_name);

FILE* get_env_file(const char* var_name, const char* mode);

struct BinString {
    std::unique_ptr<uint8_t[]> head;
    size_t len = 0;
};

BinString get_env_hex(const char* var_name);
