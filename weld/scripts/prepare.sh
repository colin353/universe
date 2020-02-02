#!/bin/bash

mkdir -p $HOME/.x20/data/largetable
mkdir -p $HOME/codefs

# Unmount the codefs directory if it is mounted
findmnt -rno SOURCE,TARGET ~/codefs
if [ $? -eq 0 ]; then
  fusermount -u -z $HOME/codefs
fi
