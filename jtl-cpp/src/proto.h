#pragma once

#include <cstdio>
#include <cstdint>

char* get_env(const char* var_name);

int get_env_int(const char* var_name);

FILE* get_env_file(const char* var_name, const char* mode);

struct BinString {
    uint8_t* head = nullptr;
    size_t len = 0;
    ~BinString();
};

BinString get_env_hex(const char* var_name);
