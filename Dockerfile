FROM rust:latest
RUN rustup target install x86_64-pc-windows-gnu

RUN apt-get update -y && \
    apt-get install -y openssh-client \
        python3 python3-pip \
        mingw-w64 \
        python3-venv
    
RUN python3 -m venv --system-site-packages /env
RUN . /env/bin/activate
RUN . /env/bin/activate && pip install maturin matplotlib numpy pandas ipython  tree-sitter tree-sitter-rust mkdocs patchelf
RUN cargo install cargo-nextest --locked
RUN rustup component add clippy