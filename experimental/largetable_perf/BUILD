package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")

rust_binary(
    name = "largetable_perf",
    srcs = [
        "main.rs",
    ],
    deps = [
        "//largetable:largetable_client",
        "//weld",
    ],
)
