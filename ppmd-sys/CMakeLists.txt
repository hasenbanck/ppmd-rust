cmake_minimum_required(VERSION 3.10)
project(libppmd C)

set(CMAKE_C_STANDARD 17)
set(CMAKE_C_STANDARD_REQUIRED ON)

set(CMAKE_C_FLAGS_RELEASE "-O3")
set(CMAKE_BUILD_TYPE Release)

set(PPMD_SOURCES
    libppmd/CpuArch.c
    libppmd/Ppmd7.c
    libppmd/Ppmd7aDec.c
    libppmd/Ppmd7Dec.c
    libppmd/Ppmd7Enc.c
    libppmd/Ppmd8.c
    libppmd/Ppmd8Dec.c
    libppmd/Ppmd8Enc.c
)

add_library(ppmd STATIC ${PPMD_SOURCES})
target_include_directories(ppmd PUBLIC libppmd)
