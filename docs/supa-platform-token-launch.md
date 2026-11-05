# Official SUPA vanity mint and owner launch architecture

Owner requirements and read-only compatibility review, **5 November 2026**.
The current uppercase `SUPA` prefix supersedes the earlier suffix requirement.
Design only: no key generation/export, secret installation, metadata upload,
signing, simulation RPC, launch, Mainnet transaction or deployment is authorized.

## Fixed product requirements

| Field | Required value or boundary |
|---|---|
| Token name | `Supa Agency` |
| Symbol | `SUPA` (`$SUPA` is the display notation) |
| Blockchain | Solana; the exact live cluster requires explicit launch approval |
| Mint prefix | Exact, case-sensitive uppercase ASCII `SUPA` at the beginning of the canonical Base58 mint public key |
| Launch surface | Owner-restricted interface on verified HTTPS `supaagency.xyz`, after website publication |
| Launch mechanism | Pump.fun, through a reviewed website-integrated instruction builder |
| Signing | Explicit owner-wallet approval/signature and the pre-generated mint signer; no autonomous signing |
| Confirmation | Independently verify successful chain execution and the exact created mint before marking it official |
| Identity | One canonical official platform mint, separate from every user-created agent token |

No official mint address exists in these requirements. The prefix alone is not an
address, reservation, signature, security property or evidence of authenticity.
Do not accept a fee-payer wallet, associated token account or bonding-curve address
as the mint. The literal predicate is `canonical_mint.starts_with("SUPA")`, after
valid public-key decoding/canonical re-encoding; no trimming, Unicode substitution,
case folding or fallback to `supa`, `Supa` or `pump`. Agent tokens do not inherit the
platform's vanity prefix requirement.

## Compatibility findings and evidence

Official Pump.fun sources were retrieved at revision
`639b87fb45c3adeffa6f7c4a1bf0cb85e392bf22`, without executing downloaded code:

- [Creation contract and SDK example](https://github.com/pump-fun/pump-public-docs/blob/639b87fb45c3adeffa6f7c4a1bf0cb85e392bf22/docs/instructions/COIN_CREATION.md):
  `createV2Instruction` takes `mint: mintKeypair.publicKey`; the account table
  identifies `mint` as a signer and `user` as transaction signer/payer.
- [Program IDL](https://github.com/pump-fun/pump-public-docs/blob/639b87fb45c3adeffa6f7c4a1bf0cb85e392bf22/idl/pump.json):
  `create_v2` marks both accounts writable/signers. Its mint is Token-2022 in the
  documented creation path. The reviewed interface does not document a mandatory
  `pump` address suffix.

**Finding:** a chosen, pre-generated signer is compatible with the documented
custom SDK instruction interface. Inferring that a valid `SUPA` vanity signer can
supply that mint is reasonable, but is not an executed integration or on-chain
acceptance test. No hosted Pump.fun UI import support was verified. Use the official
SDK/program integration rather than assuming an iframe or third-party launch API
can import a signer. Do not send a private key to a builder or launch API.

Local `web/src/components/console.tsx` still provides a disabled launch concept;
there is no executable integrated flow or vanity-signer custody adapter. Exact SDK
package, dependency hashes, IDL and current deployed-program compatibility must be
reviewed/pinned together before implementation. Do not assume Devnet support.
Current program identity from the pinned IDL is
`6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P`; this is evidence, not an active allowlist.
Raw-source checksums and sanitized account findings are retained privately under
`artifacts/supa-platform-token-research/verification.json`.

## Proposed generation and custody boundary

Generation requires a separate approval after website acceptance. Prepare a
reviewed, version-pinned local tool using vetted Ed25519/OS-CSPRNG primitives.
On an isolated owner-controlled device, search independent random candidates and
retain only a key whose public address satisfies the exact prefix. Bound CPU,
wall time and cancellation; exhaustion must stop without weakening the prefix.
Leading Base58 characters of fixed-width Solana public keys are not uniformly
distributed. Do not reuse a suffix search estimate for this prefix requirement;
any performance estimate needs a separately approved generator assessment. Do not truncate a key, edit an
address, use weak deterministic seeds or purchase a key from a vanity service.

The match must go directly into an authenticated-encrypted local keystore with
restrictive permissions, atomic creation and no overwrite. No plaintext key file,
stdout, clipboard, shell argument, dotenv, browser storage, frontend bundle, CI
artifact, API/database field, Railway variable or model context may contain it.
Choose and review the encryption/KDF backend, recovery method and custody operator
before implementation. Require independent encrypted recovery and a verified local
restore drill before using the key. Public records contain only public address,
algorithm/tool version, verification result and an opaque custody reference.

Review core dumps, swap, backups, diagnostic capture and dependency telemetry;
request best-effort secret-memory clearing without claiming perfect erasure on a
general-purpose OS. Stock CLI grinding is not an approved substitute: the inspected
[Agave source](https://github.com/anza-xyz/agave/blob/master/keygen/src/keygen.rs)
exposes vanity/case settings but writes matching keypairs to files in its ordinary
output path. No generator command is supplied or executed here.

Keep the funded owner wallet separate from the mint-creation key. The proposal is
a manually invoked, one-shot local mint co-signer with no automatic network
submission or background service, not a raw private key in the application backend.
Do not assume a hardware wallet or KMS can import arbitrary vanity key material;
verify the selected custody backend before adopting it. The mint creation signer
is also distinct from the token program's mint authority, creator identity and
metadata authority; verify the actual post-launch authorities separately.

## Website authorization and transaction signing

1. After publication, authenticate the legitimate platform owner at the exact
   verified HTTPS origin. Bind the approved wallet to that identity with a fresh,
   expiring, domain-bound challenge. A wallet connection or agent-owner/admin role
   alone grants no platform launch authority. Preserve CSRF/session protections,
   private operational controls and separate platform permission/epoch admission.
2. Prepare a single expiring intent binding token name/symbol, exact candidate mint,
   cluster/genesis identity, owner/creator/payer keys, metadata URI/content hash,
   custody reference, program/IDL/package hashes, instruction flags, fee/rent
   ceilings, transaction lifetime and one-use request ID. Store no private material.
   Reserve only the approved launch budget; do not borrow agent operating funds.
3. Build locally decoded, allowlisted instructions through the reviewed Pump.fun
   adapter. Bound/decode the supported message version; reject unresolved address
   lookup tables. Reject an altered mint, unexpected signer/program, hidden transfer,
   initial buy, fee sharing, holder-reward or other economic opt-in outside the
   approved scope. Simulation/reads and metadata uploads require their applicable
   authorization before execution. Builder output is untrusted until validated.
4. Freeze the serialized transaction message and SHA-256 fingerprint. Display the
   decoded intent and costs to the owner. Use a sign-only wallet API, never an
   automatic sign-and-send path. The owner wallet signs explicitly; the
   local co-signer independently validates the same message, approval, owner
   signature, prefix and fee bounds before the owner authorizes its mint signature.
   Return signatures/signed transaction bytes only, never a key or seed.
5. Verify all required signatures against identical message bytes before the
   separately authorized submission. Any wallet mutation or expired blockhash
   invalidates this signing attempt; collect fresh approval/signatures for changed
   bytes after reconciling prior submission status. Never silently refresh and sign.

This uses the coordination principles in Solana's
[partial-signing documentation](https://solana.com/docs/core/transactions/partial-signing).
The owner wallet adapter, transaction version and manual transfer between website
and local co-signer need an offline compatibility rehearsal. No generic signer
endpoint, executable transaction builder, wallet request or submission handler is
implemented by this document. Follow Solana's
[production-signing custody guidance](https://solana.com/docs/core/transactions/signing-in-production)
when reviewing the concrete backend.

## Confirmation, permanence and independent accounting

A submission response, signature string or Pump.fun page is insufficient. Require
the approved cluster, successful transaction status at the chosen finality policy
(proposed: finalized), exact message/signers, expected program instruction and mint,
and verified mint/program/metadata/bonding-curve state. Recheck name, symbol, URI,
prefix, supply/decimals/extensions and authority arrangements against the approved
scope. Retain sanitized receipts and public transaction references with timestamps.
No live RPC verification is performed during this assessment.

The future platform registry needs an immutable singleton official binding,
append-only attempt history, transactional uniqueness and restart reconciliation.
A pending candidate is not official. Reject conflicting confirmations; unknown
outcomes block retries/new candidate launches until reconciled. Never release a
financial reservation just because confirmation timed out. No automatic retries.

Keep platform and agent identities in distinct namespaces while enforcing global
network/mint non-aliasing across them. Confirmed $SUPA cannot silently change, be
assigned to an agent or be overwritten during recovery. The exceptional successor
rule for agent tokens does not grant a $SUPA relaunch or successor authorization.
Operational USDC revenue, launch expenses and token-market flows remain separately
accounted. A mint association creates no wallet control, holder-balance migration,
revenue right or other financial claim.

## Website-publication-first sequence and remaining approvals

1. Resolve the existing frontend security/publication gate; separately authorize
   website publication, DNS/TLS/origin checks and acceptance at `supaagency.xyz`.
2. Separately approve the generator/custody implementation, key ceremony and
   independent encrypted recovery. Nothing in this review creates a key.
3. Implement/test inert owner authorization, canonical-mint registry, message
   validation and local co-signing with synthetic public fixtures/mocked signatures.
   Prove prefix case, wrong keys/bytes, expiry, duplicate requests, unknown outcomes,
   owner isolation and recovery rejection before any real signing.
4. Review exact owner wallet, cluster, fees, metadata, authorities, program/package
   identities and no-unapproved-buy/economic-option policy; obtain fresh explicit
   approval for one intent, each required signature and its submission.
5. Launch only through that published owner-restricted site after separate
   authorization, then independently verify and record the original canonical mint.

Do not bypass website publication by using a terminal launch or hosted fallback.
The current status is **NO-GO for generation, signing and launch execution**.
No website publication, DNS change, Railway change, key handling, token issuance,
Mainnet transaction or X publishing is authorized by this documentation work.
