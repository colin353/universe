load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

# Rust rules
http_archive(
    name = "rules_rust",
    sha256 = "93955cfc4232aeec69273f3b3c57b7055924f7515ba48f1b77ec37a259cd9943",
    strip_prefix = "rules_rust-994f8de6889f2e2631cf3ea3aa9fe21a4612fd06",
    urls = [
        "https://github.com/colin353/rules_rust/archive/994f8de6889f2e2631cf3ea3aa9fe21a4612fd06.tar.gz",
    ],
)

load("@rules_rust//rust:repositories.bzl", "rules_rust_dependencies", "rust_register_toolchains")
rules_rust_dependencies()
rust_register_toolchains(version="nightly", iso_date="2022-02-23", edition="2018")

load("@rules_rust//proto:repositories.bzl", "rust_proto_repositories")
rust_proto_repositories()

load("@rules_rust//proto:transitive_repositories.bzl", "rust_proto_transitive_repositories")
rust_proto_transitive_repositories()


# Go rules

http_archive(
    name = "io_bazel_rules_go",
    sha256 = "f2dcd210c7095febe54b804bb1cd3a58fe8435a909db2ec04e31542631cf715c",
    urls = [
        "https://mirror.bazel.build/github.com/bazelbuild/rules_go/releases/download/v0.31.0/rules_go-v0.31.0.zip",
        "https://github.com/bazelbuild/rules_go/releases/download/v0.31.0/rules_go-v0.31.0.zip",
    ],
)

http_archive(
    name = "bazel_gazelle",
    sha256 = "de69a09dc70417580aabf20a28619bb3ef60d038470c7cf8442fafcf627c21cb",
    urls = [
        "https://mirror.bazel.build/github.com/bazelbuild/bazel-gazelle/releases/download/v0.24.0/bazel-gazelle-v0.24.0.tar.gz",
        "https://github.com/bazelbuild/bazel-gazelle/releases/download/v0.24.0/bazel-gazelle-v0.24.0.tar.gz",
    ],
)

load("@io_bazel_rules_go//go:deps.bzl", "go_register_toolchains", "go_rules_dependencies")
load("@bazel_gazelle//:deps.bzl", "gazelle_dependencies", "go_repository")

############################################################
# Define your own dependencies here using go_repository.
# Else, dependencies declared by rules_go/gazelle will be used.
# The first declaration of an external repository "wins".
############################################################

go_rules_dependencies()

go_register_toolchains(version = "1.18")

gazelle_dependencies()

# Python rules

load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

rules_python_version = "740825b7f74930c62f44af95c9a4c1bd428d2c53" # Latest @ 2021-06-23

http_archive(
    name = "rules_python",
    sha256 = "09a3c4791c61b62c2cbc5b2cbea4ccc32487b38c7a2cc8f87a794d7a659cc742",
    strip_prefix = "rules_python-{}".format(rules_python_version),
    url = "https://github.com/bazelbuild/rules_python/archive/{}.zip".format(rules_python_version),
)

# Docker rules
http_archive(
    name = "io_bazel_rules_docker",
    sha256 = "85ffff62a4c22a74dbd98d05da6cf40f497344b3dbf1e1ab0a37ab2a1a6ca014",
    strip_prefix = "rules_docker-0.23.0",
    urls = ["https://github.com/bazelbuild/rules_docker/releases/download/v0.23.0/rules_docker-v0.23.0.tar.gz"],
)

load(
    "@io_bazel_rules_docker//repositories:repositories.bzl",
    container_repositories = "repositories",
)
container_repositories()

load("@io_bazel_rules_docker//repositories:deps.bzl", container_deps = "deps")

container_deps()

load(
    "@io_bazel_rules_docker//container:container.bzl",
    "container_pull",
)

container_repositories()
container_deps()

# 

# Vendored node
http_archive(
    name = "vendored_node",
    build_file_content = """exports_files(["node-v14.8.0-linux-x64/bin/node"])""",
    sha256 = "c7761fe5d56d045d1540b1f0bc8a20d7edf03e6fd695ee5fbffc1dd9416ccc75",
    urls = ["https://nodejs.org/dist/v14.8.0/node-v14.8.0-linux-x64.tar.xz"],
)

load("@io_bazel_rules_docker//rust:image.bzl", _rust_image_repos = "repositories")
load("@io_bazel_rules_docker//container:pull.bzl", "container_pull")
container_pull(
    name = "glibc_base",
    digest = "sha256:32c93ea6e867f4deee92912656c77f78f50e6e3d031dbfd85270dd30d75ed1ff",
    registry = "gcr.io",
    repository = "distroless/cc-debian10",
)
container_pull(
    name = "build_base",
    digest = "sha256:cd987774f7e27ffc46cc13312c1c9e2df469ea87b2b17682590a91d0342e9544",
    registry = "registry.hub.docker.com",
    repository = "colinmerkel/build",
)
