package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_binary")
load("@rules_rust//rust:defs.bzl", "rust_test")
load("@rules_rust//rust:defs.bzl", "rust_library")
load("@rules_rust//proto:proto.bzl", "rust_proto_library")
load("@rules_rust//proto:proto.bzl", "rust_grpc_library")
load("//util/bus:bus.bzl", "rust_bus_library")

rust_bus_library(
    name = "metal_bus",
    srcs = ["metal.bus"],
)

rust_library(
    name = "core",
    srcs = ["core.rs"],
    deps = [
        ":metal_bus",
    ]
)

rust_library(
    name = "load_balancer",
    srcs = ["load_balancer.rs"],
    deps = [
        "//third_party:futures",
        "//third_party:hyper",
        "//third_party:tokio",
        "//third_party:native_tls",
        "//third_party/vendor/tokio-native-tls-0.3.1:tokio_tls",
    ],
)

rust_library(
    name = "config",
    srcs = ["config.rs"],
    data = ["metal.ccl"],
    deps = [
        ":metal_bus",
        "//util/ccl",
        "//util/ggen",
    ],
)

rust_test(
    name = "config_test",
    crate = ":config",
    # Not sure why this isn't inherited from the other target...
    data = ["metal.ccl"],
)

rust_library(
    name = "state",
    srcs = ["state.rs"],
    deps = [
        ":metal_bus",
        "//util/bus",
    ],
)

rust_library(
    name = "service",
    srcs = ["service.rs"],
    deps = [
        ":metal_bus",
        "//util/bus",
        ":core",
        ":state",
        ":diff",
    ],
)

rust_library(
    name = "diff",
    srcs = ["diff.rs"],
    deps = [
        ":metal_bus",
    ],
)

rust_library(
    name = "monitor",
    srcs = [
        "monitor.rs",
        "process.rs"
    ],
    deps = [
        ":core",
        ":metal_bus",
        "//util/rainbow",
        "//third_party:libc",
        "//third_party:sha256",
        "//third_party:futures",
        "//third_party:tokio",
        "//third_party:hyper",
        "//third_party:hyper_tls",
    ],
)

rust_test(
    name = "service_test",
    crate = ":service",
)

rust_binary(
    name = "cli",
    srcs = [
        "cli.rs",
    ],
    deps = [
        ":config",
        ":diff",
        ":core",
        ":metal_bus",
        "//util/flags",
        "//third_party:futures",
        "//util/bus:bus_rpc",
        "//util/bus",
        "//util/cli",
        "//third_party:hyper",
    ],
)

rust_binary(
    name = "metal",
    srcs = [ 
        "main.rs",
    ],
    deps = [
        ":service",
        ":state",
        ":monitor",
        ":metal_bus",
        ":load_balancer",
        "//util/bus:bus_rpc",
        "//util/flags",
        "//third_party:tokio",
        "//third_party:rand",
        "//third_party:native_tls",
        "//third_party:futures",
    ]
)

rust_binary(
    name = "gateway",
    srcs = [
        "gateway.rs",
    ],
    deps = [
        ":metal_bus",
        "//auth:auth_client",
        "//util/bus:bus_rpc",
        "//util/flags",
        "//third_party:tokio",
        "//third_party:rand",
        "//third_party:futures",
        "//third_party:hyper",
    ]
)

