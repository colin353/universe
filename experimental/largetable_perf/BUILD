package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_binary")

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
        "//util/sstable",
        "//weld",
    ],
)
