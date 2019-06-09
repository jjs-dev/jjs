cmake_minimum_required(VERSION 3.7.0)
set(SRC_FILES src/jtl.cpp)
set(HEADER_FILES include/jtl.h)
add_library(jtl STATIC ${SRC_FILES})
target_include_directories(jtl PUBLIC include)
set_target_properties(jtl PROPERTIES PUBLIC_HEADER ${HEADER_FILES})
install(TARGETS jtl
        ARCHIVE DESTINATION lib
        PUBLIC_HEADER DESTINATION include)
