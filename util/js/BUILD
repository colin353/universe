package(default_visibility = ["//visibility:public"])

load("//tools/fec:fec.bzl", "fe_library")
load("//tools/fec:fec.bzl", "fe_test")

fe_library(
    name = "store",
    srcs = ["store.mjs"],
)

fe_library(
    name = "debounce",
    srcs = ["debounce.mjs"],
)

fe_library(
    name = "truncate",
    srcs = ["truncate.mjs"],
)

fe_test(
    name = "truncate_test",
    srcs = [ "truncate_test.mjs" ],
    deps = [ ":truncate" ],
)

fe_library(
    name = "human_date",
    srcs = ["human_date.mjs"],
)

fe_test(
    name = "human_date_test",
    srcs = [ "test_human_date.mjs" ],
    deps = [ ":human_date" ],
)
