package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")
load("@rules_rust//proto:proto.bzl", "rust_proto_library")
load("@rules_rust//proto:proto.bzl", "rust_grpc_library")
load("@io_bazel_rules_docker//rust:image.bzl", "rust_image")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")

rust_image(
    name = "chat_server_img",
    base = "@glibc_base//image",
    binary = ":chat",
)

docker_push(
    name = "chat_server_img_push",
    image = ":chat_server_img",
    registry = "us.gcr.io",
    repository = "mushu-194218/chat",
    tag = "dev",
)

rust_binary(
    name = "chat",
    srcs = ["chat.rs"],
    edition = "2018",
    deps = [
        ":chat_grpc_rust",
        ":chat_service",
        ":irc",
        "//util/flags",
        "//third_party:futures",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:protobuf",
        "@rules_rust//proto/raze:tls_api",
        "@rules_rust//proto/raze:tls_api_stub",
    ],
)

rust_library(
    name = "irc",
    srcs = ["irc.rs"],
    edition = "2018",
    deps = [
        ":chat_grpc_rust",
        ":chat_service",
        "//third_party:futures",
    ],
)

rust_library(
    name = "chat_service",
    srcs = ["chat_service.rs"],
    edition = "2018",
    deps = [
        ":chat_grpc_rust",
        "//third_party:futures",
        "@rules_rust//proto/raze:grpc",
    ],
)

rust_library(
    name = "chat_client",
    srcs = ["client.rs"],
    edition = "2018",
    deps = [
        ":chat_grpc_rust",
        "//util/grpc_tls",
        "@rules_rust//proto/raze:grpc",
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
