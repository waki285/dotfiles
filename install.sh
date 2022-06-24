#!/bin/bash

set -u

#...

DOTPATH=~/.dotfiles

echo "START"

for f in .??*
do
    [ "$f" = ".git" ] && continue

    cp "$f" ~/
done

echo "DONE"