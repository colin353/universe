def maybe_remove_prefix(s, prefix):
    if s.startswith(prefix):
        return s[len(prefix):]
    return s

def _implementation(ctx):
  args = [ x.path for x in ctx.files.srcs ]

  out_shell = ctx.actions.declare_file("%s.sh" % ctx.attr.name)

  files_to_copy = [] + ctx.files.srcs
  for dep in ctx.attr.deps:
    files_to_copy += dep.files.to_list()

  paths_to_copy = []
  for file in files_to_copy:
    path = maybe_remove_prefix(file.path, "bazel-out/k8-fastbuild/bin/")
    path = maybe_remove_prefix(path, "bazel-out/k8-opt/bin/")
    paths_to_copy.append(path.replace('"', "\\\""))

  index_src = [ p.path for p in ctx.attr.index.files.to_list() ][0]

  script = """
#!/bin/bash

cp $0.runfiles/%s/%s ./index.html

$0.runfiles/%s/homepage/sync/sync index.html %s
""" % (ctx.workspace_name, index_src, ctx.workspace_name, " ".join(paths_to_copy))

  ctx.actions.write(
      output = out_shell,
      content = script,
      is_executable = True,
  )

  return [
     DefaultInfo(
         executable = out_shell,
         runfiles = ctx.runfiles([
             ctx.file._sync_bin
         ] + files_to_copy + ctx.attr.index.files.to_list() )
     ),
  ]

bucket_sync = rule(
    implementation = _implementation,
    attrs = {
        "deps": attr.label_list(),
        "srcs": attr.label_list(allow_files = True),
        "index": attr.label(allow_single_file = True),
        "_sync_bin": attr.label(
            allow_single_file = True,
            default = Label("//homepage/sync:sync"),
            cfg = "target",
            executable = True
        )
    },
    executable = True,
)
