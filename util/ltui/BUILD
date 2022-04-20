package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_binary")

rust_binary(
    name = "ltui",
    srcs = [
        "main.rs",
    ],
    deps = [
        "//largetable:largetable_client",
        "//util/flags",
        "//util/init",
    ],
)
