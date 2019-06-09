cmake_minimum_required(VERSION 3.7.0)
set(SRC_FILES src/jtl.cpp)
set(HEADER_FILES include/jtl.h)
add_library(jtl STATIC ${SRC_FILES})
target_include_directories(jtl PUBLIC include)
set_target_properties(jtl PROPERTIES PUBLIC_HEADER ${HEADER_FILES})
install(TARGETS jtl
        ARCHIVE DESTINATION lib
        PUBLIC_HEADER DESTINATION include)

function(add_checker checker_name src_file)
    set(target_name checker-${checker_name})
    add_executable(${target_name} ${src_file})
    target_include_directories(${target_name} PRIVATE include)
        target_link_libraries(${target_name} PRIVATE jtl)
    install(TARGETS ${target_name}
            RUNTIME DESTINATION bin)
endfunction()
add_checker(cmp-tokens src/cmp-tokens.cpp)
