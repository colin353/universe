package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

rust_library(
    name = "tmpl",
    srcs = [
        "tmpl.rs",
    ],
)

rust_test(
    name = "tmpl_test",
    crate = ":tmpl",
)

rust_library(
    name = "escape",
    srcs = [
        "escape.rs",
    ],
)

rust_test(
    name = "escape_test",
    crate = ":escape",
)
