package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")

rust_binary(
    name = "fes",
    srcs = ["fes.rs"],
    deps = [
        "//util/flags",
        "//util/ws",
        "//third_party:tokio",
    ],
)

rust_binary(
    name = "fes_with_js",
    srcs = ["fes.rs"],
    data = [
        "//homepage/js:code_pad",
        "//homepage/js:diff_view",
        "//tools/search/js:flow_card",
        "//tools/search/js:flow_view",
        "//tools/search/js:search_box",
    ],
    deps = [
        "//util/flags",
        "//util/ws",
        "//third_party:tokio",
    ],
)
