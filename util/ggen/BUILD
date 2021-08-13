package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

rust_library(
    name = "ggen",
    srcs = [ "lib.rs", "basic.rs", "macros.rs" ],
    edition = "2018",
)

rust_test(
    name = "ggen_test",
    crate = ":ggen",
)

