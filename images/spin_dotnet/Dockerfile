FROM --platform=${BUILDPLATFORM} mcr.microsoft.com/dotnet/sdk:7.0-bullseye-slim-amd64 AS build
WORKDIR /opt/build
RUN apt-get update && apt-get install xz-utils
RUN curl -LO https://github.com/bytecodealliance/wizer/releases/download/dev/wizer-dev-x86_64-linux.tar.xz \
    && tar -xvf wizer-dev-x86_64-linux.tar.xz \
    && rm wizer-dev-x86_64-linux.tar.xz \
    && install wizer-dev-x86_64-linux/wizer /usr/local/bin

FROM build AS build-dotnet
COPY . .
RUN dotnet build -c Release 

FROM scratch
COPY --from=build-dotnet /opt/build/bin/Release/net7.0/MyTestProject.wasm ./bin/Release/net7.0/MyTestProject.wasm
COPY --from=build-dotnet /opt/build/spin.toml .