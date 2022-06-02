package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load("@rules_rust//proto:proto.bzl", "rust_proto_library", "rust_grpc_library")
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
        ":gcp",
        "//third_party:futures",
        "//third_party:hyper",
        "//third_party:hyper_tls",
        "//third_party:json",
        "//third_party:rand",
        "//third_party:tokio",
        "//util/ws",
        "//util/ws:ws_utils",
        "@rules_rust//proto/raze:grpc",
    ],
)

rust_library(
    name = "auth_client",
    srcs = ["client.rs"],
    edition = "2018",
    deps = [
        ":auth_grpc_rust",
        "//third_party:lazy_static",
        "//third_party:futures",
        "//util/cache",
        "//util/grpc_tls",
        "@rules_rust//proto/raze:grpc",
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
        "//third_party:tls_api_openssl",
        "//util/flags",
        "//util/ws",
        "//third_party:tokio",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:protobuf",
        "//third_party:tls_api",
    ],
)

rust_library(
    name = "gcp",
    srcs = ["gcp.rs"],
    edition = "2018",
    deps = [
        "//third_party:base64",
        "//third_party:hyper",
        "//third_party:json",
        "//third_party:openssl",
        "//third_party:tokio",
        "//third_party:hyper_tls",
        "//util/ws:ws_utils",
    ],
)

rust_test(
    name = "gcp_test",
    crate = ":gcp",
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
        "//third_party:tls_api_openssl",
        "//util/flags",
        "//third_party:tokio",
        "//util/ws",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:protobuf",
        "//third_party:tls_api",
    ],
)

docker_push(
    name = "auth_server_img_push",
    image = ":auth_server_img",
    registry = "us.gcr.io",
    repository = "mushu-194218/auth",
    tag = "dev",
)
