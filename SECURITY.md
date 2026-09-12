# Security Policy

## Supported versions

Security fixes are applied to the latest released minor version. The `0.1.x` line is beta and may
receive compatibility changes before `1.0.0`.

## Reporting a vulnerability

Do not open a public issue. Use GitHub's private vulnerability reporting for this repository or
contact `security@viapost.io`. Include reproduction steps, affected versions and impact. Please do
not include API keys or customer data. We will acknowledge a report within five business days.

## Credential handling

Load API keys from a secret manager or environment variable. Never commit keys, print the client
with custom wrappers that expose headers, or include credentials in issue reports. Rotate any key
that might have been exposed.

