package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")
load("@io_bazel_rules_docker//rust:image.bzl", "rust_image")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")

rust_binary(
    name = "chat",
    srcs = ["chat.rs"],
    edition = "2018",
    deps = [
        ":chat_grpc_rust",
        ":chat_service",
        ":irc",
        "//util/flags",
        "@io_bazel_rules_rust//proto/raze:futures",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
        "@io_bazel_rules_rust//proto/raze:tls_api",
        "@io_bazel_rules_rust//proto/raze:tls_api_stub",
    ],
)

rust_library(
    name = "irc",
    srcs = ["irc.rs"],
    edition = "2018",
    deps = [
        ":chat_grpc_rust",
        ":chat_service",
        "@io_bazel_rules_rust//proto/raze:futures",
    ],
)

rust_library(
    name = "chat_service",
    srcs = ["chat_service.rs"],
    edition = "2018",
    deps = [
        ":chat_grpc_rust",
        "@io_bazel_rules_rust//proto/raze:futures",
        "@io_bazel_rules_rust//proto/raze:grpc",
    ],
)

rust_library(
    name = "chat_client",
    srcs = ["client.rs"],
    edition = "2018",
    deps = [
        ":chat_grpc_rust",
        "@io_bazel_rules_rust//proto/raze:grpc",
    ],
)

proto_library(
    name = "chat_proto",
    srcs = ["chat.proto"],
)

rust_proto_library(
    name = "chat_proto_rust",
    deps = [":chat_proto"],
)

rust_grpc_library(
    name = "chat_grpc_rust",
    deps = [":chat_proto"],
)

rust_binary(
    name = "chex",
    srcs = ["chex.rs"],
    deps = [
        ":chat_client",
        "//util/flags",
    ],
)
