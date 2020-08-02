# TODO: Required etc
# Output:
# Jtl_LIBS: libraries you should link to
# Jtl_INCLUDES: add to include path
set(Jtl_HW foo)
if (DEFINED ENV{JJS_PATH})
    set(Jtl_LIBS $ENV{JJS_PATH}/lib/libjtl.a pthread m dl rt)
    set(Jtl_INCLUDES $ENV{JJS_PATH}/include/)
else ()
    message(FATAL_ERROR "$JJS_PATH env var is not set")
endif ()