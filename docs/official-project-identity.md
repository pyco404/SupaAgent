# Official project identity and domain preparation

- Project: **Supa Agent**.
- Tagline: **Born to think. Built to survive.**
- Blockchain: **Solana**.
- Official website domain: **https://trysupaagent.xyz**.
- Official X account: **https://x.com/trysupaagent**, handle **@trysupaagent**.
- Planned token name: **Supa Agent**; planned ticker: **SUPA**.

These are owner-supplied branding decisions. The domain designation does not
establish a deployed or reachable website. SUPA is not live; no token mint,
contract address, price, sale or listing is asserted. The platform's planned
ability to launch other tokens is separate from the planned SUPA token.

## Website and social metadata

The existing Next.js layout and approved terminal experience use the official
domain and X handle in metadata. Terminal navigation updates canonical and
Open Graph URLs to the selected clean path, excluding queries and fragments.
The About section links to the official website and X account using existing
terminal link styling. The existing logo assets remain unchanged.

Private/local previews remain noindex/nofollow. Canonical metadata is branding,
not permission to expose the application or remove staging authentication.
Social-card rendering on the external platforms is not verified before deployment.
The official X link is not an API integration: no credentials are connected and
autonomous publishing remains disabled.

## Future Railway domain checklist — approval required

1. Obtain separate approval for the deployment target, resources, image digests,
   spending envelope and domain attachment. The existing six-hour synthetic-only
   rehearsal does not implicitly authorize a public launch or this domain.
2. Attach `trysupaagent.xyz` only to the approved frontend in the existing Supa
   Agent project and approved environment. Keep API, worker and PostgreSQL private.
3. Read Railway's actual domain-verification and routing records for that service.
   Review the exact record types, names, values and any conflicting existing
   records with the domain administrator before separately approving DNS edits.
   No DNS targets, IP addresses or verification tokens have been guessed here.
4. After approval, verify DNS ownership, TLS issuance, HTTPS redirects, clean
   routes, authentication and canonical URLs. Review whether indexing should
   remain disabled; do not remove noindex as part of domain attachment alone.
5. Set the approved runtime origin (`SUPA_SITE_ORIGIN`) to the exact deployment
   origin only at cutover. Metadata alone does not change origin allowlists,
   cookies, API permissions or Railway configuration. Do not repoint local or
   synthetic staging configuration to the official domain now.
6. Record the prior domain/DNS configuration and a rollback procedure before
   edits. Never remove unrelated records or expose internal services.

No DNS records, Railway resources, X credentials, publishing permissions, token
launch controls or protected database records are changed by this preparation.
Persistent sensitive staging still requires verified encryption/signing identities,
independent recovery-key custody, encrypted backup and a restoration drill.
