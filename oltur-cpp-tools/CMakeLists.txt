# Standalone build for the oltur-cpp-tools C++ library.
#
# This is provided so the library can be built and reused on its own. The
# portknock Rust binary does NOT use it — its build.rs invokes the C++
# compiler directly, so `cargo build` needs no CMake.
cmake_minimum_required(VERSION 3.15)
project(oltur_cpp_tools CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(oltur_cpp_tools STATIC src/md5.cpp)
target_include_directories(oltur_cpp_tools PUBLIC include)
target_compile_options(oltur_cpp_tools PRIVATE -O2 -fno-exceptions -fno-rtti)
