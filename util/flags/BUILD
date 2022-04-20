package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_test", "rust_library")

rust_library(
    name = "flags",
    srcs = [
        "lib.rs",
        "parse.rs",
    ],
)

rust_test(
    name = "flags_test",
    crate = ":flags",
)
