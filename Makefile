rust:
	curl https://sh.rustup.rs -sSf | sh

postgresql:
	sudo apt update
	sudo apt install postgresql postgresql-contrib
	sudo apt-get install postgresql postgresql-client
	sudo systemctl start postgresql.service

cratesio:
	curl https://static.crates.io/db-dump.tar.gz --output ./data/crates.db-dump.tar.gz
	cd data && tar -xf crates.db-dump.tar.gz 


time 		:= $(shell ls data | egrep '[0-9]+-[0-9]+-[0-9]+-[0-9]+' | sort -r | head -n 1)
timewords 	:= $(subst -, ,$(time))
year 		:= $(word 1,$(timewords))
month 		:= $(word 2,$(timewords))
day 		:= $(word 3,$(timewords))
timeofday 	:= $(word 4,$(timewords))
hour		:= $(shell expr substr "$(timeofday)" 1 2)
minute		:= $(shell expr substr "$(timeofday)" 3 2)
sencond		:= $(shell expr substr "$(timeofday)" 5 2)
# year := $(time:0:4)

dropdatabaseALL:
	echo BE AWARE THAT ALL DATA IN THE DATABASE WILL BE LOST!!!
	echo DROP DATABASE IF EXISTS crates | psql -U postgres 

database: 
	createdb -U postgres crates
	cd data/$(time) && psql -U postgres crates < schema.sql
	cd data/$(time) && psql -U postgres crates < import.sql

test: 
	@echo $(time)
	@echo $(year)
	@echo $(month)
	@echo $(day)
	@echo $(timeofday)
	@echo $(hour)
	@echo $(minute)
	@echo $(sencond)
	


# clone submodule of crates.io-index and rust
submodule:
	git submodule sync
	git submodule update --init

# all: rust postgresql
# 	echo finish
