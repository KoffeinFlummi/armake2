FROM debian:10.0-slim as build

RUN apt-get update --yes
RUN apt-get install --yes --no-install-recommends libssl-dev=1.1.1c-1
RUN apt-get install --yes --no-install-recommends cargo=0.35.0-2
RUN apt-get install --yes --no-install-recommends pkg-config=0.29-6
RUN apt-get install --yes --no-install-recommends ca-certificates

WORKDIR /armake2-master
COPY . .
RUN cargo build --release

FROM debian:10.0-slim
RUN apt-get update --yes && \
    apt-get install --yes --no-install-recommends libssl-dev=1.1.1c-1 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=build /armake2-master/target/release/armake2 /usr/bin/armake2
ENTRYPOINT [ "/usr/bin/armake2" ]
