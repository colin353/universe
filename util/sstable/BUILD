package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")

proto_library(
    name = "sstable_proto",
    srcs = ["sstable.proto"],
)

rust_proto_library(
    name = "sstable_proto_rust",
    deps = [":sstable_proto"],
)

rust_library(
    name = "sstable",
    srcs = [
        "index.rs",
        "sstable.rs",
    ],
    edition = "2018",
    deps = [
        ":sstable_proto_rust",
        "//third_party:byteorder",
        "//third_party:mmap",
        "//util/itertools",
        "//util/primitive",
        "//util/shard_lib",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "sstable_test",
    crate = ":sstable",
)
