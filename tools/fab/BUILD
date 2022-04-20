package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test", "rust_binary")

rust_binary(
    name = "fab", 
    srcs = [
        "main.rs",
    ],
    deps = [
        ":fab_lib",
        "//util/flags",
    ],
    edition = "2018",
)

rust_library(
    name = "fab_lib", 
    srcs = [
        "lib.rs",
        "resolver.rs",
        "fs.rs",
        "environment.rs",
        "config.rs",
    ],
    deps = [
        "//util/ccl",
        "//util/pool",
    ],
    edition = "2018",
)

rust_test(
    name = "fab_test", 
    crate = ":fab_lib",
)
