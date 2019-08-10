# Todo before bootstrap

## Sync, merge

Right now sync status is not checked, which means you could submit an out-of-date
client and it will just corrupt the codebase.

1. Check if sync is required and block submit if not synced
2. Implement merge logic

Merge logic:

   ------> [ client ] 
   |
[ #1 ] --> [ #2 ] --> [ #3 ]

Two sets of changes: one from 1 --> client and one from 1 --> 3.

Collect up changes in the client.
For each change from baseline to now:
 - if it touches any files changed in the client, keep track

For each both modified files:
 - create a set of changes: [start line, end line]: [ add: "", remove: "", add: "", remove: "" ]
 - for each non-overlapping set of changes, apply them
 - for each overlapping set, use SCM change markers and apply both

If there are any overlapping sets, mark the file as requiring manual resolution.

### Design

Create a merge library which handles the case of [original, change1, change2] --> [merged, requires-resolution]

## Some way to mirror to github
