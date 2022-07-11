package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")

rust_library(
    name = "recordio",
    srcs = [
        "lib.rs",
    ],
    deps = [
        "//util/bus",
    ],
)

rust_test(
    name = "recordio_test",
    crate = ":recordio",
)
