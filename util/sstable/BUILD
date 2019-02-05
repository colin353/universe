package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")

proto_library(
    name = "sstable_proto",
    srcs = [ "sstable.proto" ],
    )

rust_proto_library(
    name = "sstable_proto_rust",
    deps = [ ":sstable_proto" ],
    )

rust_library(
    name = "sstable",
    srcs = [
        "sstable.rs",
        ],
    deps = [
        "//util/primitive",
        ":sstable_proto_rust",
        "@io_bazel_rules_rust//proto/raze:protobuf",
        "//third_party:byteorder",
        ]
    )

rust_test(
    name = "sstable_test",
    deps = [
        ":sstable",
        ]
    )
