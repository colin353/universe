package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")

rust_library(
    name = "rainbow",
    srcs = ["lib.rs"],
    deps = [
        "//util/gfile",
        "//auth:auth_client",
        "//util/cli",
    ],
)

rust_test(
    name = "rainbow_test",
    crate = ":rainbow",
)
