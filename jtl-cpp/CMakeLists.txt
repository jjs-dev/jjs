cmake_minimum_required(VERSION 3.7.0)
project(Jtl VERSION 0.1.0)
set(SRC_FILES src/jtl.cpp src/testgen.cpp src/proto.cpp src/checker.cpp src/valuer.cpp src/util.cpp)
set(HEADER_FILES include/jtl.h include/testgen.h include/checker.h include/valuer.h)

set(TEMP_DIR ${CMAKE_CURRENT_BINARY_DIR})

# Integrate with cargo

set(RAND_FFI_HEADER ${CMAKE_CURRENT_BINARY_DIR}/rand-ffi.h)
set(CRATE ${CMAKE_CURRENT_SOURCE_DIR}/rand-ffi)
set(RUST_TARGET ${CMAKE_CURRENT_SOURCE_DIR}/../target)
if (${CMAKE_BUILD_TYPE} EQUAL "Debug")
    set(RUST_OPT_ARGS --debug)
    set(RUST_TARGET ${RUST_TARGET}/debug)
else ()
    set(RUST_OPT_ARGS --release)
    set(RUST_TARGET ${RUST_TARGET}/release)
endif ()
add_custom_target(JtlRsHeaders
        COMMAND cbindgen ${CRATE} -o ${RAND_FFI_HEADER} -c ${CMAKE_CURRENT_SOURCE_DIR}/cbindgen.toml)
add_custom_target(JtlRsBuild
        COMMAND cargo build ${RUST_OPT_ARGS} WORKING_DIRECTORY ${CRATE})
add_custom_target(JtlRs)
add_custom_command(TARGET JtlRs COMMAND ${CMAKE_COMMAND} -E copy ${RUST_TARGET}/librand_ffi.a ${CMAKE_CURRENT_BINARY_DIR}/libJtlRsAr.a)
add_dependencies(JtlRs JtlRsBuild JtlRsHeaders)
# JTL itself
add_library(Jtl STATIC ${SRC_FILES})
target_include_directories(Jtl PUBLIC include src ${CMAKE_CURRENT_BINARY_DIR})
add_dependencies(Jtl JtlRs)
set_target_properties(Jtl PROPERTIES PUBLIC_HEADER "${HEADER_FILES}")
target_link_libraries(Jtl PUBLIC pthread rt dl m)

install(TARGETS Jtl
        ARCHIVE DESTINATION /tmp
        PUBLIC_HEADER DESTINATION include/jjs)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/libJtl.a DESTINATION lib RENAME libjtl.a)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/libJtlRsAr.a DESTINATION lib RENAME libjtlrs.a)
install(FILES ${CMAKE_CURRENT_SOURCE_DIR}/JtlConfig.cmake DESTINATION share/cmake)

# Checkers
function(add_builtin builtin_name src_file)
    set(target_name builtin-${builtin_name})
    add_executable(${target_name} ${src_file})
    target_include_directories(${target_name} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include)
    target_link_options(${target_name} PUBLIC -L${CMAKE_CURRENT_BINARY_DIR})
    target_link_libraries(${target_name} PUBLIC Jtl JtlRsAr)
    add_dependencies(${target_name} Jtl)
    install(TARGETS ${target_name}
            RUNTIME DESTINATION bin)
endfunction()
add_builtin(checker-cmp-tokens src/builtin/cmp-tokens.cpp)
add_builtin(valuer-icpc src/builtin/val-icpc.cpp)