package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

rust_library(
    name = "markdown",
    srcs = [
        "lib.rs",
    ],
    edition = "2018",
)

rust_test(
    name = "markdown_test",
    crate = ":markdown",
)
