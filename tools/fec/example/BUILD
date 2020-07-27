load("//tools/fec:fec.bzl", "fe_component")
load("//tools/fec:fec.bzl", "fe_application")
load("//tools/fec:fec.bzl", "fe_devenv")

fe_component(
    name = "custom_button",
    srcs = [
        "custom_button.css",
        "custom_button.html",
        "custom_button.js",
    ],
)

fe_application(
    name = "application",
    srcs = [
        "application_main.html",
        "application_main.js",
    ],
    deps = [
        ":custom_button",
    ],
)

fe_devenv(
    name = "application_devenv",
    srcs = ["index.html"],
    deps = [
        ":application",
        ":custom_button",
    ],
)
