# Contributing

Thank you for your interest in contributing to Containerd Shim Spin!

For general guidance, see the [SpinKube contribution documentation](https://www.spinkube.dev/docs/contrib/).

## Commit signatures

All commits must be signed and show as **Verified** on GitHub. See GitHub's guide to [signing commits](https://docs.github.com/en/authentication/managing-commit-signature-verification/signing-commits) for instructions on setting up and using a signing method.

## Development environment

The Makefile is optimized for a Linux environment and CI. However, a Linux environment is not needed to run the following commands to install cross, run unit tests, cross-compile the shim to `x86_64-unknown-linux-musl`, and run the integration tests inside a cluster:

```sh
make install-cross
make unit-tests
make build
make integration-tests
```

## Getting help

If you need help contributing, reach out in the `#spinkube-dev` channel on [the SpinKube Slack](https://cloud-native.slack.com/archives/C06PC7JA1EE).
