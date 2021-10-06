package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")


rust_library(
    name = "dns",
    srcs = [
        "lib.rs",
        "packet.rs",
    ],
)

rust_test(
    name = "dns_test",
    crate = ":dns",
)

rust_binary(
    name = "example",
    srcs = [ "example.rs" ],
    deps = [ ":dns" ],
    edition = "2018",
)
