ARG STAGE="test"

FROM scratch AS bin-test
COPY ./.tmp /

FROM scratch AS bin-release
ARG TARGETPLATFORM
COPY ./.tmp/${TARGETPLATFORM} /

FROM bin-${STAGE} AS bin
FROM kindest/node:v1.35.0@sha256:452d707d4862f52530247495d180205e029056831160e22870e37e3f6c1ac31f

# copy shims into /usr/local/bin
COPY --from=bin / /usr/local/bin/
