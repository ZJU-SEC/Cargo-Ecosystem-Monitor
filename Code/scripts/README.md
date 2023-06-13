# Guide

This directory includes SQL scripts to analyze the Crates database. Different SQL file names represent different data. There are also import scripts and dump file to construct the corresponding DB table.

To use these scripts, you should not directly run each scripts. In each sql file, there are many seperate query sql codes. You should run them seperately.

Be aware that some scripts depend on prebuild tables, you should build them in `prebuild.sql` file.

Lastly, make sure you read and understand the documentation to avoid corrupting your data.