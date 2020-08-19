package(default_visibility = ["//visibility:public"])

load("//tools/fec:fec.bzl", "fe_component")
load("//tools/fec:fec.bzl", "fe_application")
load("//tools/fec:fec.bzl", "fe_devenv")

fe_component(
    name = "code_pad",
    srcs = [
        "code_pad.css",
        "code_pad.html",
        "code_pad.js",
    ],
)

fe_application(
    name = "app_main",
    srcs = [
        "app_main.html",
        "app_main.js",
    ],
    deps = [
        ":code_pad",
    ],
)

fe_devenv(
    name = "main_devenv",
    srcs = ["index.html"],
    deps = [
        ":app_main",
    ],
)
