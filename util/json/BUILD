package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

rust_library(
    name = "json",
    srcs = [ "lib.rs", "ast.rs" ],
    deps = [ "//util/ggen" ],
    edition = "2018",
)

rust_test(
    name = "json_test",
    crate = ":json",
)

