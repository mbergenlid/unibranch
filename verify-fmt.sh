#!/bin/bash


if [[ -z $(git status --porcelain --untracked-files=no) ]]; then
    cargo fmt

    if [[ -z $(git status --porcelain --untracked-files=no) ]]; then
        exit 0
    else
        echo "Not formatted properly"
        exit 1
    fi

else
    echo "Directory is not clean"
    exit 1
fi




