# Command-line TLC runner. Compatible with classic `docker build`: no BuildKit
# syntax, cache mounts, secrets, or host volume mounts are used.
FROM eclipse-temurin:21-jre-jammy

ARG TLA_VERSION=1.7.4
ARG TLA_TOOLS_SHA1=bee4a54f3ee3d4afc347c3240ec2d9e93b075104
ARG TLA_DEB_ARCH=arm64

RUN test "$(dpkg --print-architecture)" = "${TLA_DEB_ARCH}" \
    && apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && curl -fsSL \
       "https://github.com/tlaplus/tlaplus/releases/download/v${TLA_VERSION}/tla2tools.jar" \
       -o /opt/tla2tools.jar \
    && echo "${TLA_TOOLS_SHA1}  /opt/tla2tools.jar" | sha1sum -c - \
    && apt-get purge -y --auto-remove curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /model
COPY formal/ /model/

ENTRYPOINT ["java", "-XX:+UseParallelGC", "-jar", "/opt/tla2tools.jar"]
CMD ["-workers", "2", "-config", "VrrCore.cfg", "VrrCore.tla"]
