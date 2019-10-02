#if defined(__GNUC__) || defined(__clang__)
#define ATTR_MUST_USE __attribute__((warn_unused_result))
#elif defined(_MSC_VER) && (_MSC_VER >= 1700)
#define ATTR_MUST_USE _Check_result
#else
#define ATTR_MUST_USE
#endif