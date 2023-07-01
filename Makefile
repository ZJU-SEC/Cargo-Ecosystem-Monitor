env:
	apt-get install -y postgresql
	apt-get install -y ninja-build build-essential pkg-config libssl-dev
	apt-get install -y cmake curl vim python3 git pip zip
#	apt-get install postgresql postgresql-client postgresql-contrib

rust:
	curl https://sh.rustup.rs -sSf | sh -s -- -y

postgresql_version := $(shell ls /etc/postgresql)
postgresql:
	service postgresql start
	su postgres -c "psql -c \"ALTER USER postgres PASSWORD 'postgres'\""
	cp ./config/pg_hba.conf /etc/postgresql/$(postgresql_version)/main/pg_hba.conf
	echo "listen_addresses='*'" >> /etc/postgresql/$(postgresql_version)/main/postgresql.conf
	service postgresql restart

cratesio:
	curl https://static.crates.io/db-dump.tar.gz --output ./data/crates.db-dump.tar.gz
	cd data && tar -xf crates.db-dump.tar.gz 

extract_cratesio_once:
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

import_rawdata:
	createdb -U postgres crates
	psql -U postgres crates < ./data/alltables_20220811.sql


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
	git submodule update --init crates.io-index
	git submodule update --init rust


# Set correct commits of Rust index.
setindex:
	cd crates.io-index && git checkout `git rev-list -n 1 --first-parent --before="$(year)-$(month)-$(day) $(hour):$(minute):$(sencond) +0000" master`

# all: rust postgresql
# 	echo finish
