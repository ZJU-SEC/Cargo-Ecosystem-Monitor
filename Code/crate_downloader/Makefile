private_usage:
	 nohup sh -c 'http_proxy=http://127.0.0.1:15777 https_proxy=http://127.0.0.1:15777 && env && cargo run > /dev/null 2>&1' &

private_usage_output:
	 nohup sh -c 'export http_proxy=http://127.0.0.1:15777 https_proxy=http://127.0.0.1:15777 && env && cargo run' &

private_usage2:
	http_proxy=http://127.0.0.1:15777 https_proxy=http://127.0.0.1:15777 nohup cargo run > /dev/null 2>&1 &

private_usage2_output:
	http_proxy=http://127.0.0.1:15777 https_proxy=http://127.0.0.1:15777 nohup cargo run &