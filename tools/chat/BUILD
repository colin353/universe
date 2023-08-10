package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")
load("@rules_rust//proto:proto.bzl", "rust_proto_library")
load("@rules_rust//proto:proto.bzl", "rust_grpc_library")

rust_binary(
    name = "chat",
    srcs = ["chat.rs"],
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
    deps = [
        ":chat_grpc_rust",
        ":chat_service",
        "//third_party:futures",
    ],
)

rust_library(
    name = "chat_service",
    srcs = ["chat_service.rs"],
    deps = [
        ":chat_grpc_rust",
        "//third_party:futures",
        "@rules_rust//proto/raze:grpc",
    ],
)

rust_library(
    name = "chat_client",
    srcs = ["client.rs"],
    deps = [
        ":chat_grpc_rust",
        "//util/grpc_tls",
        "//third_party:futures",
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
