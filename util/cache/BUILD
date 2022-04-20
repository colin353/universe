package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

rust_library(
    name = "cache",
    srcs = [
        "cache.rs",
    ],
)

rust_test(
    name = "cache_test",
    crate = ":cache",
)
