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
    name = "extract",
    srcs = ["extract.rs"],
    edition = "2018",
    deps = [
        ":extract_lib",
        "//util/flags",
    ],
)
