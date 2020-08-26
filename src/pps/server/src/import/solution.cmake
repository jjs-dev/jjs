project(Solution)
cmake_minimum_required(VERSION 3.12)

add_executable(Out main.cpp)
target_compile_options(Out PUBLIC -O2)