# Usage

## Online
To run online, please config `main.rs`:
```rust
run(workers: usize, todo_status: &str)
```
here:
- workers: number of threads
- todo_status: status to be processed ("undone", "fail")

## Offline
To run offline, please config `main.rs`:
```rust
run_offline(workers: usize, todo_status: &str, home: &str)
```
here:
- workers: number of threads
- todo_status: status to be processed ("undone", "fail")
- home: where source files are stored