package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")
load("@io_bazel_rules_docker//rust:image.bzl", "rust_image")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")

proto_library(
    name = "auth_proto",
    srcs = ["auth.proto"],
)

rust_proto_library(
    name = "auth_proto_rust",
    deps = [":auth_proto"],
)

rust_grpc_library(
    name = "auth_grpc_rust",
    deps = [":auth_proto"],
)

rust_library(
    name = "auth_service_impl",
    srcs = ["auth_service_impl.rs"],
    deps = [
        ":auth_grpc_rust",
        "//third_party:futures",
        "//third_party:hyper",
        "//third_party:hyper_tls",
        "//third_party:json",
        "//third_party:rand",
        "//third_party:tokio",
        "//util/ws",
        "//util/ws:ws_utils",
        "@io_bazel_rules_rust//proto/raze:grpc",
    ],
)

rust_library(
    name = "auth_client",
    srcs = ["client.rs"],
    deps = [
        ":auth_grpc_rust",
        "@io_bazel_rules_rust//proto/raze:grpc",
    ],
)

rust_binary(
    name = "server",
    srcs = [
        "server.rs",
    ],
    deps = [
        ":auth_grpc_rust",
        ":auth_service_impl",
        "//third_party:native_tls",
        "//third_party:tls_api_openssl",
        "//util/flags",
        "//util/ws",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
        "@io_bazel_rules_rust//proto/raze:tls_api",
    ],
)

rust_image(
    name = "auth_server_img",
    srcs = [
        "server.rs",
    ],
    base = "@glibc_base//image",
    deps = [
        ":auth_grpc_rust",
        ":auth_service_impl",
        "//third_party:native_tls",
        "//third_party:tls_api_openssl",
        "//util/flags",
        "//util/ws",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
        "@io_bazel_rules_rust//proto/raze:tls_api",
    ],
)

docker_push(
    name = "auth_server_img_push",
    image = ":auth_server_img",
    registry = "us.gcr.io",
    repository = "mushu-194218/auth",
    tag = "dev",
)
