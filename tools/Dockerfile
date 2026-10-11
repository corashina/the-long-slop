FROM rust@sha256:6ff07edce8775d0f64be7aba9197229407301bddf2054d62c27b541a6238a181
RUN rustup target add wasm32-unknown-unknown && rustup component add rustfmt clippy \
    && cargo install wasm-bindgen-cli --version 0.2.129 --locked --root /opt/wasm-bindgen
ENV PATH="/opt/wasm-bindgen/bin:${PATH}"
ENV CARGO_HOME=/cargo
ENV CARGO_TARGET_DIR=/target
WORKDIR /workspace
