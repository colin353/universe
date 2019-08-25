load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")

rust_library(
    name = "pool",
    srcs = [
        "pool.rs",
    ],
)

rust_binary(
    name = "pool_example",
    srcs = [
        "pool_example.rs",
    ],
    deps = [
        ":pool",
    ],
)
