package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

rust_library(
    name = "language_specific",
    srcs = [
        "default.rs",
        "lib.rs",
    ],
    edition = "2018",
    deps = [
        "//tools/search:search_proto_rust",
    ],
)

rust_test(
    name = "language_specific_test",
    crate = ":language_specific",
)
