cmake_minimum_required(VERSION 3.15)

project(lib-bdd-bindings)

# --- Settings -----------------------------------------------------------------

if(PROJECT_IS_TOP_LEVEL)
    set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
endif()

# This could be derived from the $<CONFIG> if desired.
set(LIB_BDD_CARGO_PROFILE "release" CACHE STRING "Sets the build profile used for Rust dependencies (`dev`, `release`, or `release-with-debug`)")
set_property(CACHE LIB_BDD_CARGO_PROFILE PROPERTY STRINGS "dev" "release" "release-with-debug")

# Mark some cmake variables as advanced since they don't have to be exposed to the user.
mark_as_advanced(FORCE
    CORROSION_BUILD_TESTS
    CORROSION_DEV_MODE
    CORROSION_NATIVE_TOOLING
    CORROSION_NO_WARN_PARSE_TARGET_TRIPLE_FAILED
    CORROSION_RESPECT_OUTPUT_DIRECTORY
    CORROSION_VERBOSE_OUTPUT
    Rust_RESOLVE_RUSTUP_TOOLCHAINS)


# --- Build Dependencies -------------------------------------------------------

# Use FetchContent to acquire Corrosion for the cmake integration of Rust projects.
include(FetchContent)

FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG v0.5.0
)
FetchContent_MakeAvailable(Corrosion)

find_program(CBINDGEN_EXECUTABLE
    NAMES cbindgen
    DOC "Requires cbindgen to generate the lib-bdd-ffi header"
    REQUIRED)


# --- Main Targets -------------------------------------------------------------

# Generate lib-bdd.h using cbindgen
set(lib_bdd_include ${PROJECT_BINARY_DIR}/include)
set(lib_bdd_h ${lib_bdd_include}/lib-bdd.h)
file(GLOB_RECURSE lib_bdd_ffi_sources CONFIGURE_DEPENDS ${PROJECT_SOURCE_DIR}/src/*.rs)
add_custom_command(
    OUTPUT ${lib_bdd_h}
    COMMAND ${CBINDGEN_EXECUTABLE}
    ARGS --output ${lib_bdd_h} ${PROJECT_SOURCE_DIR}
    DEPENDS ${lib_bdd_ffi_sources} ${PROJECT_SOURCE_DIR}/cbindgen.toml
    COMMENT "Generating lib-bdd.h using cbindgen"
    VERBATIM)
# Custom target such that targets in other directories can depend on the header
add_custom_target(lib-bdd-ffi-header ALL DEPENDS ${lib_bdd_h})

# Import the lib-bdd-ffi crate specifically
corrosion_import_crate(
    MANIFEST_PATH Cargo.toml
    PROFILE ${LIB_BDD_CARGO_PROFILE}
    CRATES lib-bdd-ffi
    CRATE_TYPES cdylib staticlib)
add_dependencies(lib_bdd_ffi lib-bdd-ffi-header)
target_include_directories(lib_bdd_ffi INTERFACE ${lib_bdd_include})

add_library(lib-bdd ALIAS lib_bdd_ffi)
