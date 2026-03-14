ARG STAGE="test"

FROM scratch AS bin-test
COPY ./.tmp /

FROM scratch AS bin-release
ARG TARGETPLATFORM
COPY ./.tmp/${TARGETPLATFORM} /

FROM bin-${STAGE} AS bin

FROM rancher/k3s:v1.35.2-k3s1

# copy shims from target directory into the /bin
COPY --link --from=bin / /bin/
