package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

rust_library(
    name = "cli",
    srcs = [
        "cli.rs",
    ],
    edition = "2018",
    deps = [
        "//auth:auth_client",
        "//third_party:rand",
    ],
)

rust_test(
    name = "cli_test",
    crate = ":cli",
)
