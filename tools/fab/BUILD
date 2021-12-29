package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")

rust_binary(
    name = "fab", 
    srcs = [
        "main.rs",
    ],
    deps = [
        ":fab_lib",
        "//util/flags",
    ],
    edition = "2018",
)

rust_library(
    name = "fab_lib", 
    srcs = [
        "lib.rs",
        "resolver.rs",
        "fs.rs",
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
    crate = ":fab_lib",
)
