cmake_minimum_required(VERSION 3.12)
project(JjsMinionFfiExample)

set(MINION_HEADER ${CMAKE_CURRENT_BINARY_DIR}/minion_ffi.h)
set(CRATE ${CMAKE_SOURCE_DIR}/..)
add_custom_command(OUTPUT ${MINION_HEADER}
        COMMAND cbindgen ${CRATE} -o ${MINION_HEADER})
add_custom_target(minion_ffi
        COMMAND cargo build WORKING_DIRECTORY ${CRATE})
add_executable(example main.c ${MINION_HEADER})
target_include_directories(example PUBLIC ${CMAKE_CURRENT_BINARY_DIR})
target_link_libraries(example PUBLIC ${CMAKE_SOURCE_DIR}/../../target/debug/liblibminion_ffi.a pthread rt dl m)
add_dependencies(example minion_ffi)