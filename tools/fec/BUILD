package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

rust_binary(
    name = "fec",
    srcs = ["fec.rs"],
    edition = "2018",
    deps = [
        ":fec_lib",
        "//util/flags",
    ],
)

rust_library(
    name = "fec_lib",
    srcs = ["lib.rs"],
    edition = "2018",
    deps = [":fs"],
)

rust_library(
    name = "fs",
    srcs = ["fs.rs"],
    edition = "2018",
    deps = [],
)
