# Guide

This directory includes PostgreSQL scripts to analyze the Crates database. Different SQL file names represent different data. There are also import scripts and dump files to construct the corresponding DB table.

To use these scripts, you should not directly run each script. In each sql file, there are many separate query sql codes. You should run them separately.

Be aware that some scripts depend on prebuild tables, you should build them in `prebuild.sql` file.

Lastly, make sure you read and understand the documentation to avoid corrupting your data.
