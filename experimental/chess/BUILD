package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

rust_binary(
    name = "game",
    srcs = [
        "game.rs",
    ],
    edition = "2018",
    deps = [
        ":chess_engine",
        "//third_party:rand",
    ],
)

rust_library(
    name = "chess_engine",
    srcs = [
        "engine.rs",
    ],
    edition = "2018",
    deps = [],
)

rust_test(
    name = "engine_test",
    crate = ":chess_engine",
)
