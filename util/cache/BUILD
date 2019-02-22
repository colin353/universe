package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

rust_library(
    name = "cache",
    srcs = [
        "cache.rs",
    ],
)

rust_test(
    name = "cache_test",
    deps = [
        ":cache",
    ],
)
