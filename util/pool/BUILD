package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary")

rust_library(
    name = "pool",
    srcs = [
        "pool.rs",
        "queue.rs",
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
