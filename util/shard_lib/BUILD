package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

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
