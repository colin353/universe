package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load("//util/bus:bus.bzl", "rust_bus_library")

rust_bus_library(
    name = "service",
    srcs = ["service.bus"],
)

rust_binary(
    name = "daemon",
    srcs = ["daemon.rs"],
    deps = [
      ":daemon_service",
      "//util/bus",
      "//util/bus:bus_rpc",
      "//util/flags",
      ":service",
      "//third_party:tokio",
    ],
)

rust_library(
    name = "daemon_service",
    srcs = ["daemon_service.rs"],
    deps = [
      ":service",
      "//tools/largetable:managed_largetable",
      "//util/bus",
      "//util/bus:bus_rpc",
      "//util/sstable/v3:sstable",
      "//third_party:libc",
      ":core",
    ],
)

rust_test(
    name = "daemon_test",
    crate = ":daemon_service",
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
      "lcs.rs"
    ],
)
