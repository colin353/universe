package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

rust_library(
    name = "shard_lib",
    srcs = [
        "lib.rs",
    ],
)

rust_test(
    name = "shard_lib_test",
    crate = ":shard_lib",
)
