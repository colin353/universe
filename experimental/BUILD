package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")

rust_binary(
    name = "read_recordio",
    srcs = [
        "read_recordio.rs",
    ],
    deps = [
        "//largetable:largetable_proto_rust",
        "//util/recordio",
    ],
)

rust_binary(
    name = "read_sstable",
    srcs = [
        "read_sstable.rs",
    ],
    deps = [
        "//largetable:largetable_proto_rust",
        "//third_party:rand",
        "//util/flags",
        "//util/sstable",
        "//util/sstable/v2:sstable2",
    ],
)

rust_binary(
    name = "manually_fix_x20",
    srcs = [
        "manually_fix_x20.rs",
    ],
    edition = "2018",
    deps = [
        "//util/recordio",
        "//x20:x20_grpc_rust",
    ],
)
