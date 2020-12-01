package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

rust_binary(
    name = "convert",
    srcs = ["convert.rs"],
    deps = [
        ":sstable2",
        "//util/flags",
        "//util/primitive",
        "//util/sstable",
    ],
)

rust_library(
    name = "sstable2",
    srcs = [
        "index.rs",
        "sstable2.rs",
    ],
    edition = "2018",
    deps = [
        "//third_party:byteorder",
        "//third_party:mmap",
        "//util/itertools",
        "//util/primitive",
        "//util/shard_lib",
        "//util/sstable:sstable_proto_rust",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "sstable_test",
    crate = ":sstable2",
)
