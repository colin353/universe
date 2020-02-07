package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

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
