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

Examples:

  Original  Remote  Local
1 A         A       A
2 B                 B
3 C                 K
4 D                 D
5 E                 L
6 F         F       F

Diff chunks:

Remote: (2-6): ""
Locall: (3-4): "K", (5-6): "L"

ALL of these overlap, and the SCM change markers should look like:

A
>>>>>>> LOCAL:
B
K
D
L
======= REMOTE:
<<<<<<<<
F


Method: 

Pop lowest chunk and remember start/end. If overlaps, expand to fit both overlapping chunks,
and check whether the OTHER side overlaps now. If so, repeat until it doesn't overlap anymore.

Reconstruct A and B between the new start/end. Then create a new diffchunk with the change markers
which encompasses the entire range, and add that to the list to apply.

## Some way to mirror to github
