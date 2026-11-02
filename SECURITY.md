# Security

## Reporting a vulnerability

Please do not open a public issue for a security problem. Report it privately on GitHub: open the
repository's **Security** tab and choose **Report a vulnerability**. Only you and the developer
see the report. Please include what an attacker could do, the steps to reproduce it, and the
Panopin version and system you used.

## Supported versions

Only the newest release gets security fixes. Every copy keeps itself up to date: Panopin checks
for a new version at start and every 6 hours and installs it by itself, so a fix reaches everyone
without them doing anything.

## How updates are protected

- Panopin installs only updates signed with the project's update key, and only for the version
  the update file announces, so an older or altered release cannot be passed off as a new one.
- Every Mac build is signed with the project's own certificate. Windows builds are not
  code-signed yet.
- Each release lists the SHA-256 of every file in `SHA256SUMS.txt`.

Details are in [docs/SIGNING.md](docs/SIGNING.md). What Panopin keeps, and when it goes online, is
described under [Privacy](README.md#privacy) in the README.
