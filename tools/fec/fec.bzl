OriginalSourceFiles = provider()
ModuleSourceFiles = provider()

def _component_impl(ctx):
    args = [x.path for x in ctx.files.srcs]

    out_js = ctx.actions.declare_file("%s.mjs" % ctx.attr.name)

    args.append("--output=%s" % out_js.path)

    ctx.actions.run(
        inputs = ctx.files.srcs,
        outputs = [out_js],
        arguments = args,
        progress_message = "fec: building frontend component...",
        executable = ctx.file._compiler,
    )

    original_srcs = []
    for dep in ctx.attr.deps:
        original_srcs += dep[OriginalSourceFiles].files.to_list()
    original_srcs += ctx.files.srcs

    module_srcs = []
    for dep in ctx.attr.deps:
        module_srcs += dep[ModuleSourceFiles].files.to_list()

    return [
        DefaultInfo(files = depset([out_js] + module_srcs)),
        ModuleSourceFiles(files = depset([out_js] + module_srcs)),
        OriginalSourceFiles(files = depset(original_srcs)),
    ]

fe_component = rule(
    implementation = _component_impl,
    attrs = {
        "deps": attr.label_list(),
        "srcs": attr.label_list(allow_files = True),
        "_compiler": attr.label(
            allow_single_file = True,
            default = Label("//tools/fec"),
            cfg = "target",
            executable = True,
        ),
    },
)

def _application_impl(ctx):
    args = [x.path for x in ctx.files.srcs]

    out_js = ctx.actions.declare_file("%s_apponly.mjs" % ctx.attr.name)
    args.append("--output=%s" % out_js.path)

    ctx.actions.run(
        inputs = ctx.files.srcs,
        outputs = [out_js],
        arguments = args,
        progress_message = "fec: building frontend component...",
        executable = ctx.file._compiler,
    )

    combined_js = ctx.actions.declare_file("%s.js" % ctx.attr.name)

    joined_inputs = [x for x in ctx.files.deps]
    joined_inputs.append(out_js)
    ctx.actions.run_shell(
        inputs = joined_inputs,
        outputs = [combined_js],
        command = "cat %s >> %s" % (" ".join([x.path for x in joined_inputs]), combined_js.path),
    )

    original_srcs = []
    for dep in ctx.attr.deps:
        original_srcs += dep[OriginalSourceFiles].files.to_list()
    original_srcs += ctx.files.srcs

    module_srcs = []
    for dep in ctx.attr.deps:
        module_srcs += dep[ModuleSourceFiles].files.to_list()

    return [
        OriginalSourceFiles(files = depset(original_srcs)),
        ModuleSourceFiles(files = depset(module_srcs)),
        DefaultInfo(files = depset([combined_js])),
    ]

fe_application = rule(
    implementation = _application_impl,
    attrs = {
        "deps": attr.label_list(),
        "srcs": attr.label_list(allow_files = True),
        "_compiler": attr.label(
            allow_single_file = True,
            default = Label("//tools/fec"),
            cfg = "target",
            executable = True,
        ),
    },
)

def maybe_remove_prefix(s, prefix):
    if s.startswith(prefix):
        return s[len(prefix):]
    return s

def _devenv_impl(ctx):
    out_shell = ctx.actions.declare_file("%s.sh" % ctx.attr.name)

    joined_inputs = [x for x in ctx.files.srcs]

    original_srcs = []
    for dep in ctx.attr.deps:
        original_srcs += dep[OriginalSourceFiles].files.to_list()

    module_srcs = []
    for dep in ctx.attr.deps:
        module_srcs += dep[ModuleSourceFiles].files.to_list()

    relative_paths = []
    for path in [x.path for x in module_srcs] + [x.path for x in ctx.files.srcs]:
        path = maybe_remove_prefix(path, "bazel-out/k8-fastbuild/bin/")
        path = maybe_remove_prefix(path, "bazel-out/k8-opt/bin/")
        relative_paths.append(path)

    relative_dirs = ["/".join(x.split("/")[:-1]) for x in relative_paths]

    script = """
#!/bin/bash

COMPILED_TMP_DIR=$(mktemp -d)

tools/fec/fec $(printf "%s" | sed -e "s*__BZL_PREFIX__*$1/*g") --output=$COMPILED_TMP_DIR/ --prefix=$1
printf "%s" | sed -e "s*__BZL_PREFIX__*$1/*g" | entr -p tools/fec/fec $(printf "%s" | sed -e "s*__BZL_PREFIX__*$1/*g") --output=$COMPILED_TMP_DIR/ --prefix=$1 &

TMP_DIR=$(mktemp -d)
sh -c 'cd $1; mkdir -p %s' -s $1

printf "%s" | sed -e "s*__BZL_PREFIX__*$1/*g" | entr sh -c 'cd $1; cp --no-preserve=mode,ownership --parents %s $2' -s $1 $TMP_DIR &
tools/fes/fes --base_dir=$COMPILED_TMP_DIR,$TMP_DIR
kill $(jobs -p)
rm -rf $TMP_DIR $COMPILED_TMP_DIR
    """ % (
        # Do the initial build of all assets
        " ".join(["__BZL_PREFIX__" + x.path for x in original_srcs]),
        # Build watch the input javascript files
        "\\n".join(["__BZL_PREFIX__" + x.path for x in original_srcs]),
        " ".join(["__BZL_PREFIX__" + x.path for x in original_srcs]),
        # Create all the dirs
        " ".join(relative_dirs),
        # Copy the input HTML into the runfiles dir
        "\\n".join(["__BZL_PREFIX__" + x for x in relative_paths]),
        " ".join(relative_paths),
    )

    ctx.actions.write(
        output = out_shell,
        content = script,
        is_executable = True,
    )

    return [DefaultInfo(
        executable = out_shell,
        runfiles = ctx.runfiles([
            ctx.file._server,
            ctx.file._compiler,
        ] + module_srcs),
    )]

fe_devenv = rule(
    implementation = _devenv_impl,
    attrs = {
        "deps": attr.label_list(),
        "srcs": attr.label_list(allow_files = True),
        "data": attr.label(
            default = Label("//tools/fec"),
            executable = True,
            cfg = "target",
        ),
        "_compiler": attr.label(
            allow_single_file = True,
            default = Label("//tools/fec"),
            cfg = "target",
            executable = True,
        ),
        "_server": attr.label(
            allow_single_file = True,
            default = Label("//tools/fes"),
            cfg = "target",
            executable = True,
        ),
    },
    executable = True,
)

def _fe_library_impl(ctx):
    original_srcs = []
    for dep in ctx.attr.deps:
        original_srcs += dep[OriginalSourceFiles].files.to_list()
    original_srcs += ctx.files.srcs

    out_js = ctx.actions.declare_file("%s.mjs" % ctx.attr.name)

    module_srcs = []
    for dep in ctx.attr.deps:
        module_srcs += dep[ModuleSourceFiles].files.to_list()

    ctx.actions.run_shell(
        inputs = module_srcs + ctx.files.srcs,
        tools = [ctx.file._compiler],
        command = "cp -R bazel-out/k8-*/bin/* . && cat %s > %s && %s %s" % (
            " ".join([x.path for x in ctx.files.srcs]),
            out_js.path,
            ctx.file._compiler.path,
            " ".join([x.path for x in original_srcs]),
        ),
        progress_message = "node: checking library code...",
        outputs = [out_js],
    )

    module_srcs += [out_js]

    return [
        OriginalSourceFiles(files = depset([])),
        ModuleSourceFiles(files = depset(module_srcs + [out_js])),
        DefaultInfo(files = depset([out_js])),
    ]

fe_library = rule(
    implementation = _fe_library_impl,
    attrs = {
        "deps": attr.label_list(),
        "srcs": attr.label_list(allow_files = True),
        "_compiler": attr.label(
            allow_single_file = True,
            default = Label("//third_party:node"),
            cfg = "target",
            executable = True,
        ),
    },
)
