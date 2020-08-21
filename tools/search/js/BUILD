package(default_visibility = ["//visibility:public"])

load("//tools/fec:fec.bzl", "fe_library")

fe_library(
    name = "test",
    srcs = ["test.mjs"],
    deps = [":other"],
)

fe_library(
    name = "other",
    srcs = ["other.mjs"],
)
