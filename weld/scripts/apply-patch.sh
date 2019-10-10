#!/bin/bash

SPACE=$(tig space)
tig patch > /tmp/patch.txt

cd ~/Documents/code/universe
git stash
git checkout read-only
git pull
git apply /tmp/patch.txt
git add .
git commit -m "$SPACE"
git push
git checkout -
git stash pop

cd -
