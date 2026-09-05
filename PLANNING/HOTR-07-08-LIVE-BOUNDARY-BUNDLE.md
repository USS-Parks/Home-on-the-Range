# HOTR-07/08 — Capabilities and their live REST boundary

Date: 2026-09-05. Execution under the existing full STS approval.

Implement HOTR-07 capabilities first, then HOTR-08's planned loopback REST
surface, and publish them in one focused boundary commit after both gates pass.
HOTR-07 explicitly requires a running-process permission matrix; HOTR-08 is the
planned application transport. This uses the PSPR's allowance for a tightly
justified bundle, avoiding a temporary app protocol or mock-only acceptance.
It does not waive either gate, change their identifiers, or advance HOTR-09.

Required evidence: real cryptographic token generation, hashed vault verification,
user-scoped DPAPI roundtrip and distinct-account rejection, owner-only issue and
revoke, reader/contributor and accepted-record restrictions, explicit namespace
grants, forged identity rejection, and revocation on an existing HTTP connection.
Actual HTTP tests also require fixed loopback binding, Host/Origin denial,
body/depth/result/deadline/connection limits, stable redacted errors, controlled
overload, no owner HTTP routes, no-store responses, and cancellation semantics.

The service is exercised only with synthetic new vaults and credential profiles
under approved project work paths. No existing application profile, real vault,
OS account, firewall, startup registration, or unrelated service is changed.
