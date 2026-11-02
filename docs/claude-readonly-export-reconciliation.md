# Read-only procedure: first Claude evaluation versus provider exports

Prepared 2026-11-02. **Preparation only:** no provider export was requested, no inference was retried, no database was started or modified, and no support message was sent. Current application classification remains **`unknown`**; the **$0.003536 conservative reservation remains unchanged**. The separate historical **$0.004048 hold** is untouched.

## Fixed comparison target

| Field | Exact target |
| --- | --- |
| Export interval | **2026-11-01T19:20:00Z inclusive → 2026-11-01T19:23:00Z exclusive** |
| Lagos equivalent | **1 November 2026, 20:20–20:23 WAT** |
| Actual local attempt | 19:21:03.297616369–19:21:05.147692103 UTC |
| Workspace | `wrkspc_01FargC8PH6NDA25MHMRgz3F` |
| Model | `claude-haiku-5-5` |
| Application session/nonce | `f80246de-acbe-416f-b6b3-98ecbf15e286` — not a provider request ID |
| Independent budget | `3bae1525-7e83-4220-9682-d30d06cc06f2`: $0.005 ceiling; $0.003536 held |
| Original scope | `b11a1cb723e32742e23988b897504dd524e026390e027a0def5398ae44eb931b` |

The original job, agent, task, evidence, consent and expiry are historical comparison inputs, not a reusable execution authorization. Do not renew them, delete the execution journal, restart the paid command, alter provider configuration or change either hold.

## 1. Acquire evidence without generating work

The authorized owner should use their existing Console account. Confirm the selected organization owns the target workspace; record its public identifier locally. Never send credentials, API-key secret values, authentication headers, payment details or private prompts.

Obtain **both Usage and Cost exports**, retaining their original CSV/JSON files. Choose the exact interval and workspace above where supported. Also preserve an all-model/key cross-check so a model alias or incorrect key filter cannot silently hide a row. Record exact filter settings, timezone, export-generation time, data freshness/coverage, precision/units and any warning/partial-data indicator.

[Console documentation](https://support.claude.com/en/articles/9534590-cost-and-usage-reporting-in-the-claude-console) describes finer Usage chart views, CSV exports and rate-limited-request information. Cost exports may be coarser; if only daily data is available, keep it as corroboration and label its wider range. Do not claim daily totals uniquely identify this three-minute attempt.

If finer authorized administrative reporting is needed, the following is a **request specification for review, not an executable command or new access grant**. Use existing authorized access only; separate approval is required before Codex makes an authenticated administrative request. Do not create an admin key or reuse the ordinary workspace generation key as proof of administrative access.

```json
{
  "methods": "GET only",
  "paths": [
    "/v1/organizations/analytics/usage",
    "/v1/organizations/analytics/cost"
  ],
  "starting_at": "2026-11-01T19:20:00Z",
  "ending_at": "2026-11-01T19:23:00Z",
  "time_bucket_width": "1m",
  "workspace_ids": ["wrkspc_01FargC8PH6NDA25MHMRgz3F"],
  "group_by": ["workspace_id", "model", "api_key_id"]
}
```

Keep all models/keys initially and inspect their returned dimensions; do not invent an API-key identifier. Preserve every page and its metadata. Official [Analytics documentation](https://platform.claude.com/docs/en/manage-claude/usage-cost-analytics-api) covers administrative authentication, these filters/buckets, request/token counts and detailed costs. Its cost amounts use decimal **cents**, not dollar floats: parse as decimal and divide by 100. Do not sum overlapping totals, duplicate pages or token/cost groupings twice. These are aggregates, not guaranteed request-level receipts.

## 2. Verify reporting completeness and preserve evidence

The owner's existing dashboard observation is dated **2 November, 03:40 UTC / 04:40 WAT** and showed zero tokens/cost. It is useful corroboration, not a raw export or uniquely attributable request receipt.

The newer Analytics API documents refreshes typically within four hours, potentially up to 24 hours, plus later revisions. Check `data_refreshed_at` covers the **end of the target interval**; null, missing or earlier timestamps require an explicit completeness caveat. The interval and filters must remain fixed across pages. Do not assume the Console exposes the same watermark.

A useful later observation is after **2 November, 19:23 UTC / 20:23 WAT**, 24 hours after the entire comparison interval. Passing that time does **not** prove finality or authorize a refund. Retain any known freshness caveat and use provider confirmation if ambiguity remains.

Store exports only in an owner-private local directory under ignored `artifacts/claude-reconciliation/exports/`, outside Git and public reports. Use a new timestamped directory for each acquisition; preserve earlier versions. Set directory mode 0700 and files 0600. Record SHA-256 hashes, source, observation time and export settings in a private manifest. Hashing/copying evidence is allowed; opening CSVs in software that rewrites them or evaluates spreadsheet formulas is not part of this procedure. Never overwrite original files or upload them to an unapproved service.

## 3. Read-only comparison worksheet

Fill this worksheet locally without SQL writes, provider mutations or inference calls:

| Evidence item | What to record |
| --- | --- |
| Provenance | Owner export / authorized provider report / support response; observation timestamp |
| Attribution | Organization, full workspace ID, returned model/key identifiers, interval/timezone |
| Completeness | All pages, freshness timestamp, aggregation resolution, partial-data warnings |
| Processing | Provider request/message ID and status if present; otherwise explicitly unavailable |
| Usage | Exact uncached input, output, cache read/write counts and request-count semantics |
| Billing | Exact currency/unit, unrounded amount, discounts/credits treatment and reporting revisions |
| Local match | Original journal/session ID, timestamps, one intent stage, absent local usage/report |
| Conflicts | Other traffic, wider buckets, model aliases, mismatched organization or inconsistent totals |
| Conclusion | Processing confidence and billing confidence separately; unresolved gaps |

All-key/all-model aggregate zero is not equivalent to an HTTP rejection receipt. A rounded $0.00 amount or unchanged $10.00 balance can conceal sub-cent differences. Missing rows can reflect incomplete reporting or a wrong filter. A provider request ID is particularly useful because the local application nonce was not a provider correlation identifier. Do not assign another request's cost merely because it falls in the same minute.

## 4. Decision rules — no automatic accounting transitions

| Evidence result | Read-only assessment and action |
| --- | --- |
| Wrong organization, incomplete coverage, missing pages or rounding-only evidence | Remain `unknown`; retain reservation; correct the evidence collection |
| Complete fine-grained zero usage/cost, but no uniquely attributable processing evidence | Evidence favors no billable usage; document uncertainty and escalate if still ambiguous; do not release |
| Attributable provider confirmation of rejection before inference and zero charge | Prepare an evidence-backed zero-charge accounting proposal for separate approval; leave current records untouched |
| Attributable completed generation with authoritative usage/cost | Prepare a cost-reconciliation proposal for separate approval; do not claim missing work output or quality was recovered |
| Provider cannot locate/uniquely identify the request, or records conflict | Remain `unknown`; retain hold and replay fence; preserve the support case |

Keep provider processing status and provider billing status as separate assessment fields. No script, worksheet or support reply in this procedure changes application state. Any release, settlement, refund or increase requires sufficient evidence **and new explicit approval of the exact transition**.

## 5. Inconclusive evidence and support escalation

If coverage remains inconclusive after the reporting checkpoint, record:

> The supplied exports do not uniquely establish the processing or billing outcome of the November 1 request. Local records lack the upstream request ID/status and authoritative usage. Application status remains unknown; 3536 microUSD remains conservatively reserved. No accounting action or retry is authorized.

Prepare the [unsent support inquiry](claude-provider-support-inquiry-draft.md). Add only verified export/organization details and redacted evidence. A separate authorization is required before Codex sends anything; the owner may review the draft themselves. Do not submit raw API logs, environment files, database backups or private request material. If support cannot identify the attempt, retain that uncertainty rather than infer an unbilled rejection.

A new paid evaluation requires a new reviewed scope/nonce and fresh paid authorization; this read-only procedure does not consume or renew permission for any model request.

## Validation of this preparation

Public reporting documentation was checked. Original scope/attempt file hashes and all 26 protected primary/backup/accounting hashes match the prior baseline; primary PostgreSQL remained stopped. No export has been supplied or retrieved in this step, so no export-based reconciliation result is claimed. Only documentation changes are committed individually. Private exports, manifests and sensitive support attachments must remain untracked. Secrets scanning and the complete one-file history audit apply; no push or deployment is authorized.
