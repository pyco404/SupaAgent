# Frontend and future integrations

Milestone 4 implements Next.js, TypeScript and Tailwind CSS in `web/`, following the user's updated design direction: near-black, restrained mint, pixel typography throughout, dotted grids, thin neutral borders and an editorial technical layout. The live structural reference was inspected with REA; its prose and illustrations were not copied. The original pixel mascot and interactive survival diagram are code-native SVG/CSS. See [implementation, page coverage and REA evidence](milestone-4.md). Earlier milestone documents describe the historical backend-only scope.

Required dashboard sections map to the local API:

| Approved dashboard area | Backend contract / future behavior |
|---|---|
| Agent status | Lifecycle, next-cycle behavior, last cycle and safe failure codes |
| Goals and memory | Persisted goals, completion, timestamped learning and cycle references |
| Activity | Cycle IDs, sanitized proposals, authorization decisions and execution history |
| Financial charts | Simulated ledger with explicit labels, separate Devnet wallet observations and provider USD estimates; exact unit conversion and null-runway handling |
| X activity | Drafts with source event links and human review; publishing unavailable |
| Trading controls | Disabled/unavailable until a separately authorized integration |
| Agent factory | Unavailable to autonomous tools; current creation is operator-only local setup |

The client displays simulated values as simulated on relevant cards/charts, separate from Devnet test tokens and API cost estimates. The Next.js server layer mediates authenticated GET requests through a closed path allowlist. The browser never receives the Rust bearer token. The connected website requires local operator authentication; no mutation routes are forwarded. Lifecycle controls and future executable integrations remain outside this console. Do not expose it publicly.

Milestone 3 provides Devnet identity mapping, encrypted local custody, read-only verification, network pinning and operating-reservation policies independent of LLM output; see [wallet requirements and limitations](milestone-3.md). Isolated signing and real settlement remain future work. Mainnet execution is absent. Jupiter integration will require separate financial limits, asset/route restrictions, position/exposure limits, slippage controls, quote expiry and a kill switch. The simulated ledger must never be silently converted into real balances.

X integration must retain draft provenance, semantic review, explicit publishing authorization, credential isolation, rate limits and idempotency. Draft event references are not a proof that all generated claims are true. Agent creation will need operator-approved limits and resource funding. Production code changes must remain reviewable by humans and outside runtime tool permissions.

Future provider adapters implement `Reasoner`; they must return bounded structured actions/drafts, exclude secrets from model context, preserve timeouts and enforce no retries of uncertain paid operations. Existing provider usage metering and authorized dollar-budget limits remain separate from Devnet observations. Real autonomous treasury spending needs further authorization and settlement controls.

Production readiness also requires multi-user authentication, pagination/retention, memory retrieval beyond bounded recent context, database backup/restore exercises, queue supervision, metrics/traces and stronger content/privacy controls. This milestone intentionally does not deploy infrastructure.
