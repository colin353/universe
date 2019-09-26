package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

rust_library(
    name = "tmpl",
    srcs = [
        "tmpl.rs",
    ],
)

rust_test(
    name = "tmpl_test",
    crate = ":tmpl",
)

rust_library(
    name = "escape",
    srcs = [
        "escape.rs",
    ],
)

rust_test(
    name = "escape_test",
    crate = ":escape",
)
