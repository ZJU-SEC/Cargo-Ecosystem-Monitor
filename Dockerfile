FROM ubuntu

# Set up env
ENV DEBIAN_FRONTEND=noninterative
# RUN echo $PATH
# WORKDIR /usr/src/Cargo-ecosystem-Monitor
# COPY . .
RUN apt-get update && apt-get install -y make gcc
RUN install -y postgresql
RUN install -y ninja-build build-essential pkg-config libssl-dev
RUN install -y cmake curl vim python3 git pip zip
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH=$PATH:/root/.cargo/bin
# Verify version
RUN rustc --version && cargo --version

# CMD ["make", "postgresql"]
CMD ["sh", "-c", "while true; do sleep 1; done"]