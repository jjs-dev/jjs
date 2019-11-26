#include "proto.h"
#include "jtl.h"
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <cstring>

char* get_env(const char* var_name) {
    char* res = getenv(var_name);
    if (res == nullptr) {
        die("ERROR: var %s not present\n", var_name);
    }
    return res;
}

int get_env_int(const char* var_name) {
    char* res = get_env(var_name);
    int ans;
    if (sscanf(res, "%d", &ans) == 0) {
        die("ERROR: var `%s` has value `%s`, which is not integer\n", var_name,
            res);
    }
    return ans;
}

FILE* get_env_file(const char* var_name, const char* mode) {
    int fd = get_env_int(var_name);
    FILE* file = fdopen(fd, mode);
    if (file == nullptr) {
        die("ERROR: var `%s` contains fd `%d`, which is not file of mode %s",
            var_name, fd, mode);
    }
    return file;
}

const uint8_t CHAR_BAD = 255;

uint8_t decode_hex_char(char x) {
    if ('0' <= x && x <= '9')
        return x - '0';
    if ('a' <= x && x <= 'f')
        return x - 'a' + 10;
    return CHAR_BAD;
}

BinString decode_hex(char* data) {
    size_t n = strlen(data);
    if (n % 2 != 0)
        return {};
    auto out = new uint8_t[n / 2];
    for (size_t i = 0; i < n / 2; ++i) {
        auto a = decode_hex_char(data[2 * i]);
        auto b = decode_hex_char(data[2 * i + 1]);
        if (a == CHAR_BAD || b == CHAR_BAD) {
            delete[] out;
            return {};
        }
        out[i] = a * 16 + b;
    }
    BinString bs;
    bs.len = n / 2;
    bs.head.reset(out);
    return std::move(bs);
}

BinString get_env_hex(const char* var_name) {
    char* value = get_env(var_name);

    auto res = decode_hex(value);
    if (!res.head) {
        die("ERROR: var `%s` contains '%s', which is not hex\n", var_name,
            value);
    }
    return res;
}
