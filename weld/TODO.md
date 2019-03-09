# Todo list

*Write a lot more tests for integration*: I noticed that there
are issues with some clients being created which are suddenly
based on some old stuff. No clue why. May have to do with having
several clients open in parallel.

*Get multi-user support going*: need multi-user support before
you can put review on the cloud

*Retool review code for bazel*: it was last built before bazel,
  needs to be fixed up to work correctly

  *Get review code onto the cloud*: should be straightforward

  *Find some way to bootstrap*: maybe need some way to sync changes
  back to git? Or automatically create PRs/apply changes to keep
  things synced on git? Since I don't really trust the code to be
  stored in itself yet.
