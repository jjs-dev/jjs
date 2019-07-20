#pragma once

#ifdef __GNUC__
#define FORMAT_FN(x) __attribute__ (( format( printf, x, x+1 ) ))
#else
#define FORMAT_FN(x)
#endif
