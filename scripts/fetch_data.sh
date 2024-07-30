#!/bin/sh

project_path=$(dirname "$(dirname "$0")")

# Don't use `--delete` since any error in the path could cascade unwanted deletes all over the filesystem.
# User has to delete the folder then start from scratch, if that's an issue.
rsync -av --info=progress2 kiz0.in.ohmportal.de:/home/meissnerfl73755/slurm_mon_data/ "$project_path"/data_dir