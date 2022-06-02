package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "hyper_tls",
    srcs = [
        "lib.rs",
        "client.rs",
        "stream.rs",
    ],
    deps = [
        "@rules_rust_proto__bytes__0_5_6//:bytes",
        "//third_party/vendor/native-tls-0.2.2:native_tls",
        "//third_party/vendor/tokio-native-tls-0.3.1:tokio_tls",
        "//third_party:tokio",
        "//third_party:hyper",
    ]
)
