FROM rust:1.71

RUN mkdir /meow-coverage
COPY . /meow-coverage
RUN cargo install --path /meow-coverage

ADD entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
