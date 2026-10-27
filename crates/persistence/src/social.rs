use crate::*;
impl Store {
    /// Both legacy and new drafts contribute to repetition checks; rejected/abstained text never does.
    pub async fn social_history(&self, id: Uuid) -> Result<Vec<PreviousDraft>> {
        let rows:Vec<Value>=sqlx::query_scalar("SELECT data FROM (SELECT data,created_at FROM tweet_drafts WHERE agent_id=$1 UNION ALL SELECT data,created_at FROM social_decisions WHERE agent_id=$1 AND disposition='draft') d ORDER BY created_at DESC LIMIT 20").bind(id).fetch_all(&self.pool).await.map_err(db)?;
        let mut result = vec![];
        for data in rows {
            let text = data["text"].as_str().ok_or(Error::Database)?.to_owned();
            let event_ids: Vec<Uuid> =
                serde_json::from_value(data["event_ids"].clone()).map_err(|_| Error::Database)?;
            let events = self.verified_events(id, &event_ids).await?;
            let topics: BTreeSet<_> = events
                .iter()
                .filter_map(|e| social_fact(e).map(|(topic, _, _)| topic.to_string()))
                .collect();
            result.push(PreviousDraft {
                text,
                event_ids,
                topics: topics.into_iter().collect(),
            });
        }
        Ok(result)
    }
    /// Validate again under the existing agent/cycle lock, including fresh persisted draft history.
    pub async fn finish_social(
        &self,
        id: Uuid,
        cycle: Uuid,
        action: Uuid,
        context: &SocialContext,
        decision: &SocialDecision,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let a = live(&mut tx, id, cycle).await?;
        let proposal:Value=sqlx::query_scalar("SELECT proposal FROM actions WHERE id=$1 AND cycle_id=$2 AND status='running' AND authorization_outcome='allowed'").bind(action).bind(cycle).fetch_optional(&mut *tx).await.map_err(db)?.ok_or(Error::Stale)?;
        if proposal["tool"] != "consider_tweet" || !a.capabilities.contains(&Capability::DraftTweet)
        {
            return Err(Error::Denied);
        }
        // Ownership and canonical event values are reloaded, never trusted from a provider context.
        let ids: Vec<_> = context
            .candidates
            .iter()
            .map(|c| c.event.id)
            .chain(
                context
                    .memories
                    .iter()
                    .flat_map(|m| m.source_event_ids.iter().copied()),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut events = vec![];
        for chunk in ids.chunks(8) {
            events.extend(events_by_ids_tx(&mut tx, id, chunk).await?);
        }
        let history_rows:Vec<Value>=sqlx::query_scalar("SELECT data FROM (SELECT data,created_at FROM tweet_drafts WHERE agent_id=$1 UNION ALL SELECT data,created_at FROM social_decisions WHERE agent_id=$1 AND disposition='draft') d ORDER BY created_at DESC LIMIT 20").bind(id).fetch_all(&mut *tx).await.map_err(db)?;
        let mut history = vec![];
        for data in history_rows {
            let ids: Vec<Uuid> =
                serde_json::from_value(data["event_ids"].clone()).map_err(|_| Error::Database)?;
            let sources = events_by_ids_tx(&mut tx, id, &ids).await?;
            history.push(PreviousDraft {
                text: data["text"].as_str().ok_or(Error::Database)?.into(),
                event_ids: ids,
                topics: sources
                    .iter()
                    .filter_map(|e| social_fact(e).map(|(t, _, _)| t.into()))
                    .collect(),
            });
        }
        let current = social_context(a.personality, &events, &a.memory, &history);
        let validation = decision
            .validate(context)
            .and_then(|()| decision.validate(&current));
        let disposition = if validation.is_err() {
            "rejected"
        } else if decision.choice == SocialChoice::Draft {
            "draft"
        } else {
            "abstain"
        };
        // Rejected free-form text is not retained: audit the code, not unverified or sensitive output.
        let data = if validation.is_ok() {
            serde_json::to_value(decision).map_err(|_| Error::Invalid)?
        } else {
            json!({"choice":"rejected","reason":"validation_failed"})
        };
        sqlx::query("INSERT INTO social_decisions(id,agent_id,cycle_id,action_id,data,disposition,failure_code) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(Uuid::new_v4()).bind(id).bind(cycle).bind(action).bind(data).bind(disposition).bind(validation.as_ref().err().map(ToString::to_string)).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("UPDATE actions SET status=$2,finished_at=now(),failure_code=$3 WHERE id=$1")
            .bind(action)
            .bind(if validation.is_ok() {
                "succeeded"
            } else {
                "failed"
            })
            .bind(validation.as_ref().err().map(ToString::to_string))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        event(&mut tx, id, Some(cycle), "social", disposition, false).await?;
        tx.commit().await.map_err(db)?;
        validation
    }
}
use std::collections::BTreeSet;
