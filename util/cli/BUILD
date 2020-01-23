package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")

rust_library(
    name = "cli",
    srcs = [
        "cli.rs",
    ],
    edition = "2018",
    deps = [
        "//third_party:rand",
    ],
)
