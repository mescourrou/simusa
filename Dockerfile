FROM rust:latest
RUN rustup target install x86_64-pc-windows-gnu

RUN apt-get update -y && \
    apt-get install -y openssh-client \
        python3 python3-pip \
        mingw-w64
    
RUN pip install maturin matplotlib numpy pandas ipython virtualenv tree-sitter tree-sitter-rust mkdocs patchelf --break-system-packages
RUN cargo install cargo-nextest --locked
RUN virtualenv --system-site-packages /env