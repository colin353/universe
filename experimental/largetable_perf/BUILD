package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")

rust_binary(
    name = "largetable_perf",
    srcs = [
        "main.rs",
    ],
    edition = "2018",
    deps = [
        "//largetable:largetable_client",
        "//largetable:largetable_proto_rust",
        "//third_party:rand",
        "//util/flags",
        "//util/pool",
        "//util/sstable/v2:sstable2",
        "//weld",
    ],
)
