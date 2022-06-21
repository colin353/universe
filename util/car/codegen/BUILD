package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")

rust_binary(
    name = "codegen",
    srcs = [
        "codegen.rs",
        "rust.rs",
    ],
    deps = [
        "//util/car:parser",
        "//util/flags",
    ]
)
