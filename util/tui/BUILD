package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")

rust_library(
    name = "tui",
    srcs = [
        "lib.rs",
    ],
    edition = "2018",
    deps = [
        "//third_party:libc",
    ],
)

rust_binary(
    name = "example",
    srcs = [
        "example.rs",
    ],
    edition = "2018",
    deps = [":tui"],
)
