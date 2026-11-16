# Security

Mod Mallet reads files from many untrusted sources: mods, archives,
installer scripts and logs. A file that makes it crash, hang, use unbounded
memory, or write outside its own output is a security problem and is
treated as one.

## Reporting a problem

Please do not open a public issue for a vulnerability. Use the repository's
private vulnerability reporting ("Report a vulnerability" on the Security
tab) so that a fix can be prepared before the details are public.

Include the version or commit you used, what you did, and, if possible, the
smallest file that triggers the problem.

## What to expect

Mod Mallet is pre-alpha and maintained by volunteers. Reports are read and
answered as soon as we can; there is no guaranteed response time yet.

## Design

The measures taken against malformed input are described in
[ADR 0013](docs/src/adr/0013-untrusted-input.md). Privacy is covered by
[ADR 0014](docs/src/adr/0014-offline-and-private.md): Mod Mallet works
offline and sends no data anywhere.
