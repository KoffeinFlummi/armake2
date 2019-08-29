FROM debian:bullseye-slim as build

RUN apt-get update --yes
RUN apt-get install --yes libssl-dev=1.1.1c-1
RUN apt-get install --yes cargo=0.37.0-3
RUN apt-get install --yes pkg-config=0.29-6

WORKDIR /armake2-master
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update --yes && \
    apt-get install --yes libssl-dev=1.1.1c-1 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=build /armake2-master/target/release/armake2 /usr/bin/armake2
ENTRYPOINT [ "/usr/bin/armake2" ]
