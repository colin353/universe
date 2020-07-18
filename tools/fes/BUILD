package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

rust_binary(
    name = "fes",
    srcs = ["fes.rs"],
    edition = "2018",
    deps = [
        "//util/flags",
        "//util/ws",
    ],
)
