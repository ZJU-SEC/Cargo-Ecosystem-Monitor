android:
	sudo apt-get update || true
	sudo apt-get install repo

android_another:
	REPO=$(mktemp /tmp/repo.XXXXXXXXX)
	curl -o ${REPO} https://storage.googleapis.com/git-repo-downloads/repo
	gpg --recv-keys 8BB9AD793E8E6153AF0F9A4416530D5E920F5C65
	curl -s https://storage.googleapis.com/git-repo-downloads/repo.asc | gpg --verify - ${REPO} && install -m 755 ${REPO} ~/bin/repo