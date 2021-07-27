package(default_visibility = ["//visibility:public"])

load("//tools/fec:fec.bzl", "fe_library")
load("//tools/fec:fec.bzl", "fe_devenv")
load("//tools/fec:fec.bzl", "fe_component")

fe_component(
   name = "gitique_ui",
   srcs = [
       "gitique_ui.mjs",
       "gitique_ui.html",
   ],
   deps = [
       ":settings_ui",
   ]
)

fe_component(
   name = "settings_ui",
   srcs = [
       "settings_ui.mjs",
       "settings_ui.html",
   ],
)

fe_devenv(
    name = "ui_devenv",
    srcs = [ "index.html" ],
    deps = [
        ":gitique_ui",
        ":github",
    ],
)

fe_library(
    name = "github",
    srcs = [ "github.mjs" ],
)
