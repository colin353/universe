package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test" )

rust_library(
    name = "markdown",
    srcs = [
        "lib.rs",
    ],
    edition = "2018",
)

rust_test(
    name = "markdown_test",
    crate = ":markdown",
)
