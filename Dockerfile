FROM rust:1.44.1

RUN curl -fsSL https://download.docker.com/linux/debian/gpg | apt-key add -
RUN apt-key fingerprint 0EBFCD88 \
    && apt-get update \
    && apt-get install -qq  software-properties-common \
    && add-apt-repository \
       "deb [arch=amd64] https://download.docker.com/linux/debian \
       $(lsb_release -cs) \
       stable" \
    && apt-get update \
    && apt-get install -qq docker-ce-cli

# gather sources
COPY submission-system /submission-system

# prepare clean images
RUN git clone https://github.com/icfpcontest2020/dockerfiles.git /dockerfiles

# change to rust project dir
WORKDIR /submission-system

# build system
RUN  cargo build --release

# start the submision system
ENTRYPOINT ["cargo", "run"]