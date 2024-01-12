# RUF Lifetime Analysis

We will download all release versions of Rust compiler and analyze its RUF definition to get the status of every RUF though the update of Rust compiler. This helps further analysis of RUF development and its usability and reliability.

This project will first download Rust compiler source codes and then produce its lifetime information. Then we'll further analyze the RUF information and visualize it. To get that, we have to execute python program `analysis.py` and `life.py`. 

Subprojects using command:
- `cargo run`: RUF definition extractor.
- `python3 analysis`: Abnormal RUF lifetime analysis. It generates abnormal RUF lifetime info in `feature_abnormal` DB table.
- `python3 life.py`: Visualization. It generates lifetime file in `figure.pdf`.

Before them, you should run `make env` first to build necessary environments.

If you find that `thread 'main' panicked at 'Unpack file failed: Custom { kind: UnexpectedEof, error: TarError { desc: "failed to iterate over archive", io: Error { kind: UnexpectedEof, message: "failed to fill whole buffer" } } }', src/util.rs:62:39`, please re-download this package as the download process fail to get complete gz pack of the source code.