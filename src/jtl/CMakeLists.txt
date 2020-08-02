cmake_minimum_required(VERSION 3.13.4)
project(Jtl VERSION 0.1.0)
set(CMAKE_CXX_STANDARD 17)

set(SRC_FILES src/jtl.cpp src/testgen.cpp src/proto.cpp src/checker.cpp src/util.cpp)
set(HEADER_FILES include/jtl.h include/testgen.h include/checker.h)

set(TEMP_DIR ${CMAKE_CURRENT_BINARY_DIR})

# JTL itself
add_library(Jtl STATIC ${SRC_FILES})
target_include_directories(Jtl PUBLIC include src ${CMAKE_CURRENT_BINARY_DIR} ${CMAKE_SOURCE_DIR}/deps)
set_target_properties(Jtl PROPERTIES PUBLIC_HEADER "${HEADER_FILES}")
target_link_libraries(Jtl PUBLIC pthread rt dl m)

install(TARGETS Jtl
        ARCHIVE DESTINATION /tmp
        PUBLIC_HEADER DESTINATION include/jjs)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/libJtl.a DESTINATION lib RENAME libjtl.a)
install(FILES ${CMAKE_CURRENT_SOURCE_DIR}/JtlConfig.cmake DESTINATION share/cmake)

# Checkers, valuers, testgens, etc
function(add_builtin builtin_name src_file)
    set(target_name builtin-${builtin_name})
    add_executable(${target_name} ${src_file})
    target_include_directories(${target_name} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_link_options(${target_name} PUBLIC -L${CMAKE_CURRENT_BINARY_DIR})
    target_link_libraries(${target_name} PUBLIC Jtl)
    add_dependencies(${target_name} Jtl)
    install(TARGETS ${target_name}
            RUNTIME DESTINATION bin)
endfunction()
add_builtin(checker-cmp-tokens src/builtin/checker-cmp-tokens.cpp)
add_builtin(checker-polygon-compat src/builtin/checker-polygon-compat.cpp)