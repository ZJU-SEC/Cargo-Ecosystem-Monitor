env: rust postgresql
	echo finish

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