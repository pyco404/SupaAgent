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

## Offline generation and verification contract — 7 November 2026

Architecture preparation only. The identity remains **Supa Agency / SUPA /
Solana / exact uppercase `SUPA` prefix / Pump.fun / platform-owner authority**.
The new `supa_domain::platform_token::validate_official_platform_mint_address`
reuses canonical Base58 decoding/re-encoding and requires exactly 32 decoded
bytes plus the exact prefix. It never generates or retains a key. Its synthetic
public-string test fixture is not an actual mint, an Ed25519 keypair or evidence
of signer ownership. No example candidate should be advertised as official.

Solana distinguishes an Ed25519 public key from a PDA; a correctly encoded
32-byte address alone does not prove a corresponding private signer exists.
See [account-address specification](https://solana.com/docs/core/accounts/account-structure#account-address).
The validation gates therefore remain separate:

1. **Address syntax:** canonical Base58, 32 bytes, exact case-sensitive prefix.
2. **Signer possession:** derive the public key from the held Ed25519 key and
   verify a fresh domain-separated possession signature after encrypted recovery.
3. **Owner authority:** independently authenticate the platform owner and verify
   their explicit approval binding this mint and custody receipt. An agent's
   creator role, an arbitrary wallet signature or a vanity prefix cannot confer it.
4. **Launch confirmation:** the exact approved mint must be independently verified
   on the approved chain before the public status changes from planned.

### Local generation boundary

The proposed generator is a separate, manually invoked owner-device tool, not
an API route, browser worker, GitHub job, Railway service or autonomous agent.
Prepare a reproducible build and audited dependency bundle while online; verify
their hashes before disconnecting the owner-controlled device. Generation must
have no RPC, HTTP, telemetry or cloud-generator dependency. Do not run it during
this architecture task.

Use the existing vetted `ring` Ed25519 and OS-CSPRNG primitives as the candidate
implementation direction: independent random candidates, public-key derivation,
canonical Base58 encoding, exact prefix check, then immediate encrypted retention
of the first match. Entropy failure must stop, never fall back to timestamps,
passwords, UUIDs, deterministic seeds or third-party generated keys. Never edit
the public string to force a match. Require finite nonzero candidate and elapsed-
time bounds, a small explicitly selected worker count, monotonic deadline checks
and cancellation between candidates. Exhaustion/cancellation must not weaken
the prefix, switch providers or automatically restart. No success-time guarantee
or uniform-character probability estimate is implied.

Existing `crates/providers/src/vault.rs` demonstrates `SystemRandom`, Ed25519,
ChaCha20-Poly1305, restricted file modes and recover-then-derive verification.
It is a **Devnet wallet vault**, not an approved platform-mint vault: do not relax
its network guard, reuse an agent key or store the platform key in its existing
wallet namespace. Reuse reviewed primitives, not its role/authority or an assumed
production-custody approval. No generator implementation is enabled by adding
the public-only validator.

### Owner-only encrypted retention

The candidate architecture uses a dedicated authenticated-encrypted envelope and
an owner-held independent 256-bit master key, with a fresh AEAD nonce. A raw-key
design needs no password KDF; a passphrase alternative requires separate review
of a suitable memory-hard KDF and its parameters, not a plain hash. The master
key must not sit beside the envelope, in a dotenv or in any platform service.
Select the owner-controlled custody and recovery medium before implementation;
arbitrary vanity-key import into a hardware wallet is not assumed to work.

Envelope authenticated data binds its version, algorithm, ceremony ID, intended
cluster/genesis, owner public identity and exact mint public key. Use a separate
platform-mint namespace and no secret-bearing Debug/Serialize/public API. Only
ciphertext reaches disk: a no-symlink, owner-only directory (0700), exclusively
created private files (0600), atomic no-replace publication, file/directory fsync
and explicit failure on overwrite or permission errors. A crash before successful
encrypted commit leaves no accepted candidate. A crash after commit requires
recovery/reconciliation of that exact envelope before any new search.

Do not print or export losing candidates, seeds, PKCS8 documents, master keys,
keypair arrays or decrypted envelopes. Keep them out of shell arguments, logs,
clipboard, browser storage, crash reports, source control and application storage.
Review swap/core-dump/cloud-backup exposure on the owner device. Zeroize owned
temporary buffers where supported; do not claim perfect erasure of all upstream
library, allocator or OS copies. Restrict access to the owner, including encrypted
recovery media; prove recovery before the mint is approved for future use.

### Retained-signer possession and recovery receipt

After encrypted storage, close the in-memory candidate and reopen the exact
envelope. Recover privately and independently rederive the public address. Reject
any address, envelope authenticated-data, prefix or owner mismatch. Then sign a
non-transaction, domain-separated possession challenge whose canonical bytes bind:

- `SUPA-MINT-POSSESSION-V1`, the website origin and a one-use ceremony/challenge ID;
- the exact mint public key and owner public identity;
- cluster/genesis identity, tool/dependency hash and encrypted-envelope hash;
- a fresh nonce, issue/expiry timestamps and the required `platform-token` purpose.

Verify the Ed25519 signature locally against the exact challenge bytes and mint
public key, and verify separate owner approval over the same receipt hash. Reject
replay, expiration, changed bytes, a different recovered key or an agent-token
scope. A directory path, claimed public key, boolean "owned" field or successful
Base58 validation is never a substitute. The proof shows possession at that time;
exclusive custody additionally depends on the owner-device and recovery controls.

Store/export only a strict public receipt: canonical mint, owner public key,
ceremony/challenge identifiers, timestamps, tool hashes, opaque custody reference,
envelope digest, challenge and signatures. No private envelope location or key
material goes into the website. A receipt is not a launch authorization, onchain
mint reservation or transaction signature. Repeat the recovery/possession check
after restoration and immediately before any separately authorized launch.
Cryptographic proof generation is planned, not exercised with production keys here.

### Pump.fun handoff and public status

The current official [coin-creation account table and SDK example](https://github.com/pump-fun/pump-public-docs/blob/main/docs/instructions/COIN_CREATION.md)
still describe a signer mint and a separate transaction payer, and the builder
accepts a mint public key. This supports the chosen-signer architecture; it does
not prove our integration, hosted-UI import support or current program acceptance
of a specific candidate. Repin the exact SDK, IDL and program evidence at the
future execution review. Do not execute the example's generic key generation,
combined create-and-buy path or unapproved economic options.

The owner-restricted website receives public receipt/intent data only. A local
co-signer retains the mint key, validates the frozen transaction message and
explicit owner-wallet signature, and returns only its signature after separate
approval. No seed upload, generic remote signer or automatic sign-and-send path
is introduced. Distinguish mint-creation signer custody from the token program's
post-creation mint/metadata authorities; never promise the owner retains powers
that the verified program assigns elsewhere. Follow the [Solana signing custody
guidance](https://solana.com/docs/core/transactions/signing-in-production).

Keep the public label **$SUPA — planned, not launched** during every preparation,
generation, recovery and pending-submission state. Only successful independent
onchain confirmation can establish the canonical official platform mint. Preserve
the separate immutable original-mint and successor rules for user-created agents;
this platform-only prefix validator does not constrain their mint prefixes or
grant their creators platform launch authority.

Before key handling, separately approve the concrete offline tool/custody backend,
encrypted recovery drill and one bounded generation ceremony. Before launch,
separately approve the exact owner-authorized intent, signatures, fees and Mainnet
submission. No such operation, keypair generation or signing occurred here.

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

1. Preserve website-publication-first ordering. Reverify HTTPS, owner access
   boundaries and current security acceptance at the published `supaagency.xyz`
   before launch; any website changes retain their separate approval requirement.
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
