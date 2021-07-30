package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

rust_library(
    name = "ws",
    srcs = [
        "ws.rs",
    ],
    edition = "2018",
    deps = [
        ":ws_utils",
        "//third_party:futures",
        "//third_party:hyper",
        "//third_party:rand",
    ],
)

rust_library(
    name = "ws_utils",
    srcs = ["ws_utils.rs"],
    deps = [
        "//third_party:lazy_static",
    ],
)

rust_test(
    name = "ws_utils_test",
    crate = ":ws_utils",
)

rust_binary(
    name = "ws_example",
    srcs = [
        "ws_example.rs",
    ],
    data = ["template.html"],
    deps = [
        ":ws",
        "//util/tmpl",
    ],
)
