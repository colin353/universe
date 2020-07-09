package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

rust_library(
    name = "engine",
    srcs = [
        "engine.rs",
    ],
    edition = "2018",
    deps = [],
)

rust_test(
    name = "engine_test",
    crate = ":engine",
)
