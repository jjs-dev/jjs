#include "jtl.h"
#include "proto.h"
#include <cstdlib>
#include <cstdio>
#include <cstdarg>
#include <cctype>
#include <cassert>

bool is_char_whitespace(char c) {
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
        if (nread == 0) break;
    }
    return true;
}
