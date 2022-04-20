package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

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

