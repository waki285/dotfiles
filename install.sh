#!/bin/bash

set -u

#...

DOTPATH=~/.dotfiles

echo "START"

for f in .??*
do
    [ "$f" = ".git" ] && continue

    ln -snfv "$DOTPATH/$f" "$HOME"/"$f"
done

echo "DONE"