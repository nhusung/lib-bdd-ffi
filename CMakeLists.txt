cmake_minimum_required(VERSION 3.15)

project(lib-bdd-bindings)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

include(CTest)

# Use FetchContent to acquire Corrosion for the cmake integration of Rust projects.
include(FetchContent)

FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG v0.4.7 # Optionally specify a commit hash, version tag or branch here
)
FetchContent_MakeAvailable(Corrosion)

# This could be derived from the $<CONFIG> if desired.
set(LIB_BDD_CARGO_PROFILE "release" CACHE STRING "Sets the build profile used for Rust dependencies (`dev`, `release`, or `release-with-debug`)")
set_property(CACHE LIB_BDD_CARGO_PROFILE PROPERTY STRINGS "dev" "release" "release-with-debug")

if(BUILD_SHARED_LIBS)
    set(crate_type "cdylib")
else()
    set(crate_type "staticlib")
endif()

# Import the lib-bdd-ffi crate specifically
corrosion_import_crate(
    MANIFEST_PATH Cargo.toml
    PROFILE ${LIB_BDD_CARGO_PROFILE}
    CRATES lib-bdd-ffi
    CRATE_TYPES ${crate_type})

# Generate the cbindgen headers automatically after compilation.
find_program(CBINDGEN_EXECUTABLE NAMES cbindgen DOC "Requires cbindgen to generate lib-bdd.h")

add_custom_command(TARGET cargo-build_lib-bdd-ffi
    POST_BUILD
    COMMAND ${CBINDGEN_EXECUTABLE}
    ARGS --output ${CMAKE_CURRENT_BINARY_DIR}/include/lib-bdd.h ${CMAKE_CURRENT_SOURCE_DIR})

target_include_directories(lib-bdd-ffi INTERFACE ${CMAKE_CURRENT_BINARY_DIR}/include)

# Install the shared objects generated.
if(BUILD_SHARED_LIBS)
    corrosion_install(TARGETS RUNTIME lib-bdd-ffi)
endif()

add_library(lib-bdd ALIAS lib-bdd-ffi)


# Mark some cmake variables as advanced since they don't have to be exposed to the user.
mark_as_advanced(FORCE
    CORROSION_BUILD_TESTS
    CORROSION_DEV_MODE
    CORROSION_NATIVE_TOOLING
    CORROSION_NO_WARN_PARSE_TARGET_TRIPLE_FAILED
    CORROSION_RESPECT_OUTPUT_DIRECTORY
    CORROSION_VERBOSE_OUTPUT
    Rust_RESOLVE_RUSTUP_TOOLCHAINS)
