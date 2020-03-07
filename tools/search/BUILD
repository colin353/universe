package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")
load("@io_bazel_rules_docker//rust:image.bzl", "rust_image")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")

proto_library(
    name = "search_proto",
    srcs = ["search.proto"],
)

rust_proto_library(
    name = "search_proto_rust",
    deps = [":search_proto"],
)

rust_binary(
    name = "search",
    srcs = ["search.rs"],
    edition = "2018",
    deps = [
        ":search_lib",
        ":search_proto_rust",
        "//util/flags",
    ],
)

rust_library(
    name = "search_lib",
    srcs = ["search_lib.rs"],
    edition = "2018",
    deps = [
        ":search_proto_rust",
        "//util/sstable",
    ],
)

rust_library(
    name = "extract_lib",
    srcs = ["extract_lib.rs"],
    edition = "2018",
    deps = [
        ":search_proto_rust",
        "//util/sstable",
    ],
)

rust_binary(
    name = "indexer",
    srcs = ["indexer.rs"],
    edition = "2018",
    deps = [
        ":extract_lib",
        ":indexer_lib",
        ":search_proto_rust",
        "//plume",
        "//util/flags",
    ],
)

rust_library(
    name = "indexer_lib",
    srcs = ["indexer_lib.rs"],
    edition = "2018",
    deps = [
        ":search_proto_rust",
        "//plume",
        "//tools/search/language_specific",
    ],
)

rust_test(
    name = "indexer_test",
    crate = ":indexer_lib",
)

rust_binary(
    name = "extract",
    srcs = ["extract.rs"],
    edition = "2018",
    deps = [
        ":extract_lib",
        "//util/flags",
    ],
)
