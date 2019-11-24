## Todo

1. Create a filter to store files locally (e.g. .swp) which shouldn't be committed
2. There are bugs with diff where some files which have been touched but not modified
   show up in the list of modified files
3. If you revert a file, it doesn't get removed from the diff due to the naive way
   review decides which files to show
