package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")

rust_binary(
    name = "codegen",
    srcs = [
        "codegen.rs",
        "rust.rs",
    ],
    deps = [
        "//util/bus:parser",
        "//util/flags",
    ]
)

rust_library(
    name = "test",
    srcs = [
        "test.rs",
        "test_test.rs",
    ],
    deps = [
        "//util/bus",
    ]
)

rust_test(
    name = "test_test",
    crate = "test",
)
