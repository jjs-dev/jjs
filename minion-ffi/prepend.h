#if defined(__GNUC__) || defined(__clang__)
#define ATTR_MUST_USE __attribute__((warn_unused_result))
#else
#define ATTR_MUST_USE
#endif