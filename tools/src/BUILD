package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load("//util/bus:bus.bzl", "rust_bus_library")

rust_bus_library(
    name = "service",
    srcs = ["service.bus"],
)

rust_library(
    name = "src_lib",
    srcs = ["lib.rs", "helpers.rs", "metadata.rs", "checkout.rs", "diff.rs"],
    deps = [
      ":service",
      "//util/pool",
      "//tools/largetable:managed_largetable",
      "//util/bus",
      "//util/itertools",
      "//util/bus:bus_rpc",
      "//util/sstable/v3:sstable",
      "//third_party:libc",
      ":core",
    ],
)

rust_test(
    name = "src_lib_test",
    crate = ":src_lib",
)

rust_binary(
    name = "server",
    srcs = ["server.rs"],
    deps = [
      ":server_service",
      ":service",
      "//util/bus",
      "//util/bus:bus_rpc",
      "//util/flags",
      "//third_party:tokio",
    ],
)

rust_library(
    name = "server_service",
    srcs = ["server_service.rs"],
    deps = [
      ":service",
      ":core",
      "//util/bus",
      "//tools/largetable:managed_largetable",
      "//util/sstable/v3:sstable",
      "//tools/largetable",
    ],
)

rust_test(
  name = "server_test",
  crate = ":server_service",
)

rust_library(
    name = "core",
    srcs = [
      "core.rs",
      "patience.rs",
      "lcs.rs",
      "render.rs",
      "conflict.rs"
    ],
    deps = [
      ":service",
      "//third_party:sha256",
      "//third_party:lz4",
    ]
)

rust_test(
  name = "core_test",
  crate = ":core",
)

rust_binary(
    name = "src",
    srcs = [
        "src.rs",
    ],
    deps = [
        ":cli",
        "//util/flags",
    ]
)

rust_library(
    name = "cli",
    srcs = [
        "cli.rs",
        "termui.rs",
    ],
    deps = [
        ":core",
        ":service",
        ":src_lib",
        "//util/tui",
        "//util/tui:filter",
        "//third_party/raw_tty",
        "//third_party:rand",
    ]
)

rust_binary(
    name = "integration_tests",
    srcs = [
        "integration_tests.rs",
    ],
    deps = [
      ":cli",
      ":server_service",
      ":src_lib",
      ":service",
      "//util/flags",
      "//util/tui",
      "//third_party:tokio",
      "//util/bus",
      "//util/bus:bus_rpc",
      ":core",
    ]
)
