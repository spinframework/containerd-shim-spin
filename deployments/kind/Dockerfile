ARG STAGE="test"

FROM scratch AS bin-test
COPY ./.tmp /

FROM scratch AS bin-release
ARG TARGETPLATFORM
COPY ./.tmp/${TARGETPLATFORM} /

FROM bin-${STAGE} AS bin

FROM kindest/node:v1.31.0

# copy shims into /bin
COPY --link --from=bin / /bin/
