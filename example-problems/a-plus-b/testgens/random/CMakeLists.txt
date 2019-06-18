find_package(Jtl CONFIG REQUIRED)
add_executable(Out main.cpp)
target_include_directories(Out PUBLIC ${Jtl_INCLUDES})
target_link_libraries(Out PUBLIC ${Jtl_LIBS})
