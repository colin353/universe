package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

rust_library(
    name = "ggen",
    srcs = [ "lib.rs", "basic.rs", "macros.rs" ],
    edition = "2018",
)

rust_test(
    name = "ggen_test",
    crate = ":ggen",
)

