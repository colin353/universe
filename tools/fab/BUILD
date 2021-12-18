package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")

rust_library(
    name = "fab", 
    srcs = [
        "lib.rs",
        "resolver.rs",
        "environment.rs",
        "config.rs",
    ],
    deps = [
        "//util/ccl",
        "//util/pool",
    ],
    edition = "2018",
)

rust_test(
    name = "fab_test", 
    crate = ":fab",
)
