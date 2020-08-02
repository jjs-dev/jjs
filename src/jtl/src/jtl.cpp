#include "jtl.h"
#include "proto.h"
#include <cassert>
#include <cctype>
#include <cstdarg>
#include <cstdio>
#include <cstdlib>

static bool is_char_whitespace(char c) {
    return c == ' ' || c == '\n' || c == '\t';
}

bool is_file_eof(FILE* f) {
    const int BUF_SIZE = 256;
    char buf[BUF_SIZE];
    while (true) {
        int nread = fread(buf, 1, BUF_SIZE, f);
        for (int i = 0; i < nread; ++i) {
            if (!is_char_whitespace(buf[i])) {
                return false;
            }
        }
        if (nread == 0) {
            break;
        }
    }
    return true;
}

void oom() {
    fprintf(stderr, "Out of memory");
    abort();
}

void* check_oom(void* ptr) {
    if (ptr) {
        return ptr;
    } else {
        oom();
    }
}

void die(char const* message, ...) {
    va_list v;
    va_start(v, message);
    vfprintf(stderr, message, v);
    exit(1);
}
