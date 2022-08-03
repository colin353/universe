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
      ":service",
      "//tools/largetable:managed_largetable",
    ],
)

