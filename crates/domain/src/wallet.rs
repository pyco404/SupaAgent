use crate::*;
use std::collections::BTreeMap;
pub const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SolanaNetwork {
    Devnet,
    Mainnet,
}
impl SolanaNetwork {
    pub fn name(self) -> &'static str {
        match self {
            Self::Devnet => "devnet",
            Self::Mainnet => "mainnet",
        }
    }
    pub fn usdc_mint(self) -> &'static str {
        match self {
            Self::Devnet => "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU",
            Self::Mainnet => "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        }
    }
    pub fn genesis(self) -> &'static str {
        match self {
            Self::Devnet => "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG",
            Self::Mainnet => "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d",
        }
    }
    pub fn require_devnet(self) -> Result<()> {
        if self == Self::Devnet {
            Ok(())
        } else {
            Err(Error::Denied)
        }
    }
}
pub fn validate_solana_address(s: &str) -> Result<()> {
    validate_base58(s, 32)
}
pub fn validate_solana_signature(s: &str) -> Result<()> {
    validate_base58(s, 64)
}
fn validate_base58(s: &str, size: usize) -> Result<()> {
    if s.len() > 90 {
        return Err(Error::Invalid);
    }
    let bytes = bs58::decode(s).into_vec().map_err(|_| Error::Invalid)?;
    if bytes.len() != size || bs58::encode(&bytes).into_string() != s {
        return Err(Error::Invalid);
    }
    Ok(())
}
/// Exact human-to-atomic conversion. Exponents, signs, floating point and excess precision are rejected.
pub fn parse_token_amount(s: &str, decimals: u8) -> Result<i64> {
    if decimals > 9 || s.is_empty() || s.len() > 32 {
        return Err(Error::Invalid);
    }
    let parts: Vec<_> = s.split('.').collect();
    if parts.len() > 2 || parts[0].is_empty() || !parts[0].bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::Invalid);
    }
    let frac = parts.get(1).copied().unwrap_or("");
    if (parts.len() == 2 && frac.is_empty())
        || frac.len() > decimals as usize
        || !frac.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(Error::Invalid);
    }
    let whole = parts[0].parse::<i64>().map_err(|_| Error::Invalid)?;
    whole
        .checked_mul(10i64.pow(decimals as u32))
        .and_then(|v| {
            v.checked_add(
                frac.parse::<i64>().unwrap_or(0) * 10i64.pow(decimals as u32 - frac.len() as u32),
            )
        })
        .ok_or(Error::Invalid)
}
pub fn format_token_amount(atoms: i64, decimals: u8) -> Result<String> {
    if atoms < 0 || decimals > 9 {
        return Err(Error::Invalid);
    }
    let scale = 10i64.pow(decimals as u32);
    if decimals == 0 {
        Ok(atoms.to_string())
    } else {
        Ok(format!(
            "{}.{:0width$}",
            atoms / scale,
            atoms % scale,
            width = decimals as usize
        ))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletIdentity {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub network: SolanaNetwork,
    pub address: String,
    pub owner_id: Uuid,
    pub origin: IdentityOrigin,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IdentityOrigin {
    HumanOperator,
    Agent { parent_agent_id: Uuid },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenAccountBalance {
    pub address: String,
    pub mint: String,
    pub atoms: i64,
    pub decimals: u8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChainEvidence {
    ConfiguredRpc,
    MockRpc,
}
impl ChainEvidence {
    pub fn name(self) -> &'static str {
        match self {
            Self::ConfiguredRpc => "configured_rpc",
            Self::MockRpc => "mock_rpc",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    pub evidence_source: ChainEvidence,
    pub network: SolanaNetwork,
    pub address: String,
    pub slot: i64,
    pub sol_lamports: i64,
    pub usdc_atoms: i64,
    pub token_accounts: Vec<TokenAccountBalance>,
}
/// No Deserialize or public unchecked constructor: production proofs are built by this verifier only.
#[derive(Debug, Clone, Serialize)]
pub struct VerifiedPayment {
    evidence_source: ChainEvidence,
    network: SolanaNetwork,
    wallet_address: String,
    signature: String,
    mint: String,
    amount_atoms: i64,
    slot: i64,
}
impl VerifiedPayment {
    pub fn evidence_source(&self) -> ChainEvidence {
        self.evidence_source
    }
    pub fn network(&self) -> SolanaNetwork {
        self.network
    }
    pub fn wallet_address(&self) -> &str {
        &self.wallet_address
    }
    pub fn signature(&self) -> &str {
        &self.signature
    }
    pub fn mint(&self) -> &str {
        &self.mint
    }
    pub fn amount_atoms(&self) -> i64 {
        self.amount_atoms
    }
    pub fn slot(&self) -> i64 {
        self.slot
    }
    /// Fail closed on incomplete metadata, unsupported token programs, authority changes, internal transfers or mismatched deltas.
    pub fn verify(
        evidence_source: ChainEvidence,
        network: SolanaNetwork,
        address: &str,
        signature: &str,
        status: &serde_json::Value,
        tx: &serde_json::Value,
    ) -> Result<Self> {
        validate_solana_address(address)?;
        validate_solana_signature(signature)?;
        if status["confirmationStatus"] != "finalized" {
            return Err(Error::Unconfirmed);
        }
        if status.get("err").is_none() || !status["err"].is_null() {
            return Err(Error::InvalidPayment);
        }
        let slot = tx["slot"]
            .as_i64()
            .filter(|s| *s >= 0)
            .ok_or(Error::InvalidPayment)?;
        if status["slot"].as_i64() != Some(slot)
            || tx["transaction"]["signatures"][0] != signature
            || tx["meta"].get("err").is_none()
            || !tx["meta"]["err"].is_null()
        {
            return Err(Error::InvalidPayment);
        }
        let keys = tx["transaction"]["message"]["accountKeys"]
            .as_array()
            .ok_or(Error::InvalidPayment)?;
        let keys: Vec<_> = keys
            .iter()
            .map(|v| {
                v.as_str()
                    .or_else(|| v["pubkey"].as_str())
                    .ok_or(Error::InvalidPayment)
            })
            .collect::<Result<_>>()?;
        let mut pre = BTreeMap::new();
        let mut post = BTreeMap::new();
        for (name, target) in [
            ("preTokenBalances", &mut pre),
            ("postTokenBalances", &mut post),
        ] {
            for row in tx["meta"][name].as_array().ok_or(Error::InvalidPayment)? {
                if row["mint"] != network.usdc_mint() {
                    continue;
                }
                let index = row["accountIndex"]
                    .as_u64()
                    .and_then(|n| usize::try_from(n).ok())
                    .filter(|n| *n < keys.len())
                    .ok_or(Error::InvalidPayment)?;
                let owner = row["owner"].as_str().ok_or(Error::InvalidPayment)?;
                if row["programId"] != TOKEN_PROGRAM || row["uiTokenAmount"]["decimals"] != 6 {
                    return Err(Error::InvalidPayment);
                }
                let amount = row["uiTokenAmount"]["amount"]
                    .as_str()
                    .and_then(|s| s.parse::<i64>().ok())
                    .filter(|n| *n >= 0)
                    .ok_or(Error::InvalidPayment)?;
                if target.insert(index, (owner, amount)).is_some() {
                    return Err(Error::InvalidPayment);
                }
            }
        }
        let mut owned = BTreeSet::new();
        let mut delta = 0i128;
        for index in pre
            .keys()
            .chain(post.keys())
            .copied()
            .collect::<BTreeSet<_>>()
        {
            if let (Some(a), Some(b)) = (pre.get(&index), post.get(&index))
                && a.0 != b.0
            {
                return Err(Error::InvalidPayment);
            }
            if pre.get(&index).is_some_and(|v| v.0 == address)
                || post.get(&index).is_some_and(|v| v.0 == address)
            {
                owned.insert(keys[index]);
                delta += post.get(&index).map(|v| v.1 as i128).unwrap_or(0)
                    - pre.get(&index).map(|v| v.1 as i128).unwrap_or(0);
            }
        }
        if delta <= 0 || delta > i64::MAX as i128 {
            return Err(Error::InvalidPayment);
        }
        let mut instructions: Vec<_> = tx["transaction"]["message"]["instructions"]
            .as_array()
            .ok_or(Error::InvalidPayment)?
            .iter()
            .collect();
        for group in tx["meta"]["innerInstructions"]
            .as_array()
            .ok_or(Error::InvalidPayment)?
        {
            instructions.extend(
                group["instructions"]
                    .as_array()
                    .ok_or(Error::InvalidPayment)?
                    .iter(),
            );
        }
        let mut transferred = 0i128;
        for ix in instructions {
            if ix["programId"] != TOKEN_PROGRAM {
                continue;
            }
            let info = &ix["parsed"]["info"];
            let source = info["source"].as_str();
            let dest = info["destination"].as_str();
            let involved = source.is_some_and(|s| owned.contains(s))
                || dest.is_some_and(|s| owned.contains(s));
            if !involved {
                continue;
            }
            let amount = match ix["parsed"]["type"].as_str() {
                Some("transferChecked")
                    if info["mint"] == network.usdc_mint()
                        && info["tokenAmount"]["decimals"] == 6 =>
                {
                    &info["tokenAmount"]["amount"]
                }
                Some("transfer") => &info["amount"],
                _ => return Err(Error::InvalidPayment),
            }
            .as_str()
            .and_then(|s| s.parse::<i64>().ok())
            .filter(|n| *n > 0)
            .ok_or(Error::InvalidPayment)? as i128;
            if dest.is_some_and(|d| owned.contains(d)) {
                transferred += amount;
            }
            if source.is_some_and(|s| owned.contains(s)) {
                transferred -= amount;
            }
        }
        if transferred != delta {
            return Err(Error::InvalidPayment);
        }
        Ok(Self {
            evidence_source,
            network,
            wallet_address: address.into(),
            signature: signature.into(),
            mint: network.usdc_mint().into(),
            amount_atoms: delta as i64,
            slot,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatingPurpose {
    Inference,
    Infrastructure,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpendingPolicy {
    pub locked: bool,
    pub reservations_enabled: bool,
    pub per_transaction_atoms: i64,
    pub daily_atoms: i64,
    pub destinations: BTreeSet<String>,
}
impl Default for SpendingPolicy {
    fn default() -> Self {
        Self {
            locked: true,
            reservations_enabled: false,
            per_transaction_atoms: 0,
            daily_atoms: 0,
            destinations: BTreeSet::new(),
        }
    }
}
impl SpendingPolicy {
    pub fn validate(&self) -> Result<()> {
        if self.per_transaction_atoms < 0
            || self.daily_atoms < self.per_transaction_atoms
            || self.destinations.len() > 100
        {
            return Err(Error::Invalid);
        }
        for d in &self.destinations {
            validate_solana_address(d)?;
        }
        Ok(())
    }
    // Every independent policy input is explicit; callers must supply trusted application state.
    #[allow(clippy::too_many_arguments)]
    pub fn authorize(
        &self,
        network: SolanaNetwork,
        lifecycle: Lifecycle,
        authorized: bool,
        destination: &str,
        amount: i64,
        daily_committed: i64,
        available: i64,
    ) -> Result<()> {
        self.validate()?;
        validate_solana_address(destination)?;
        if !authorized
            || network != SolanaNetwork::Devnet
            || lifecycle != Lifecycle::Active
            || self.locked
            || !self.reservations_enabled
            || !self.destinations.contains(destination)
            || amount <= 0
            || amount > self.per_transaction_atoms
            || daily_committed
                .checked_add(amount)
                .is_none_or(|n| n > self.daily_atoms)
            || amount > available
        {
            return Err(Error::FinancialPolicy);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasurySummary {
    pub network: SolanaNetwork,
    pub environment: String,
    pub asset: String,
    pub decimals: u8,
    pub observed_sol_lamports: i64,
    pub observed_usdc_atoms: i64,
    pub verified_incoming_usdc_atoms: i64,
    pub reserved_usdc_atoms: i64,
    pub inference_expense_usdc_atoms: i64,
    pub infrastructure_expense_usdc_atoms: i64,
    pub available_operating_usdc_atoms: i64,
    pub api_inference_estimate_microusd: i64,
    pub reconciled: bool,
    pub fresh: bool,
    pub observed_at: Option<DateTime<Utc>>,
    pub latest_payment_slot: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn address(byte: u8) -> String {
        bs58::encode([byte; 32]).into_string()
    }
    fn fixture() -> (String, String, serde_json::Value, serde_json::Value) {
        let owner = address(1);
        let dest = address(2);
        let source = address(3);
        let sig = bs58::encode([9; 64]).into_string();
        let row = |amount: &str| json!({"accountIndex":0,"mint":SolanaNetwork::Devnet.usdc_mint(),"owner":owner,"programId":TOKEN_PROGRAM,"uiTokenAmount":{"amount":amount,"decimals":6}});
        let tx = json!({"slot":42,"transaction":{"signatures":[sig],"message":{"accountKeys":[dest,source],"instructions":[{"programId":TOKEN_PROGRAM,"parsed":{"type":"transferChecked","info":{"source":source,"destination":dest,"mint":SolanaNetwork::Devnet.usdc_mint(),"tokenAmount":{"amount":"1500000","decimals":6}}}}]}},"meta":{"err":null,"preTokenBalances":[row("0")],"postTokenBalances":[row("1500000")],"innerInstructions":[]}});
        (
            owner,
            sig,
            json!({"slot":42,"err":null,"confirmationStatus":"finalized"}),
            tx,
        )
    }
    #[test]
    fn exact_amount_and_network_validation() {
        assert_eq!(parse_token_amount("1.000001", 6).unwrap(), 1_000_001);
        assert_eq!(format_token_amount(1_000_001, 6).unwrap(), "1.000001");
        assert_eq!(parse_token_amount("0.000000001", 9).unwrap(), 1);
        for bad in [
            "-1",
            "1e6",
            "1.0000001",
            "1.",
            " 1",
            "9223372036854775808",
            "1.2.3",
        ] {
            assert!(parse_token_amount(bad, 6).is_err(), "{bad}");
        }
        for network in [SolanaNetwork::Devnet, SolanaNetwork::Mainnet] {
            validate_solana_address(network.usdc_mint()).unwrap();
        }
        assert_ne!(
            SolanaNetwork::Devnet.usdc_mint(),
            SolanaNetwork::Mainnet.usdc_mint()
        );
        assert!(SolanaNetwork::Mainnet.require_devnet().is_err());
        assert!(validate_solana_address("0OIl").is_err());
        assert!(validate_solana_signature(&address(1)).is_err());
    }
    #[test]
    fn finalized_inbound_transfer_proof_is_exact() {
        let (owner, sig, status, tx) = fixture();
        let proof = VerifiedPayment::verify(
            ChainEvidence::MockRpc,
            SolanaNetwork::Devnet,
            &owner,
            &sig,
            &status,
            &tx,
        )
        .unwrap();
        assert_eq!(proof.amount_atoms(), 1_500_000);
        assert_eq!(proof.slot(), 42);
    }
    #[test]
    fn reject_unconfirmed_failed_wrong_network_and_incomplete_proofs() {
        let (owner, sig, status, tx) = fixture();
        let verify = |s: &serde_json::Value, t: &serde_json::Value| {
            VerifiedPayment::verify(
                ChainEvidence::MockRpc,
                SolanaNetwork::Devnet,
                &owner,
                &sig,
                s,
                t,
            )
        };
        for change in [json!("confirmed"), json!(null)] {
            let mut s = status.clone();
            s["confirmationStatus"] = change;
            assert!(matches!(verify(&s, &tx), Err(Error::Unconfirmed)));
        }
        for pointer in [
            "/slot",
            "/transaction/signatures/0",
            "/meta/err",
            "/meta/postTokenBalances/0/mint",
            "/meta/postTokenBalances/0/programId",
            "/meta/postTokenBalances/0/owner",
            "/meta/postTokenBalances/0/uiTokenAmount/decimals",
            "/meta/postTokenBalances/0/uiTokenAmount/amount",
            "/transaction/message/instructions/0/parsed/info/tokenAmount/amount",
        ] {
            let mut t = tx.clone();
            *t.pointer_mut(pointer).unwrap() = json!("invalid");
            assert!(verify(&status, &t).is_err(), "{pointer}");
        }
        let mut t = tx.clone();
        t["meta"]["innerInstructions"] = json!(null);
        assert!(verify(&status, &t).is_err());
        assert!(
            VerifiedPayment::verify(
                ChainEvidence::MockRpc,
                SolanaNetwork::Mainnet,
                &owner,
                &sig,
                &status,
                &tx
            )
            .is_err()
        );
        let mut t = tx.clone();
        t["transaction"]["message"]["instructions"] = json!([]);
        assert!(verify(&status, &t).is_err());
    }
    #[test]
    fn caps_permissions_lifecycle_and_overflow_are_independent() {
        let dest = address(3);
        let p = SpendingPolicy {
            locked: false,
            reservations_enabled: true,
            per_transaction_atoms: 5,
            daily_atoms: 8,
            destinations: [dest.clone()].into(),
        };
        let check = |network, state, auth, amount, daily, available| {
            p.authorize(network, state, auth, &dest, amount, daily, available)
        };
        assert!(check(SolanaNetwork::Devnet, Lifecycle::Active, true, 5, 3, 5).is_ok());
        for state in [
            Lifecycle::Created,
            Lifecycle::Paused,
            Lifecycle::Suspended,
            Lifecycle::Terminated,
        ] {
            assert!(check(SolanaNetwork::Devnet, state, true, 1, 0, 5).is_err());
        }
        for (auth, amount, daily, available) in [
            (false, 1, 0, 5),
            (true, 0, 0, 5),
            (true, -1, 0, 5),
            (true, 6, 0, 9),
            (true, 5, 4, 9),
            (true, 5, 0, 4),
            (true, 1, i64::MAX, 9),
        ] {
            assert!(
                check(
                    SolanaNetwork::Devnet,
                    Lifecycle::Active,
                    auth,
                    amount,
                    daily,
                    available
                )
                .is_err()
            );
        }
        assert!(check(SolanaNetwork::Mainnet, Lifecycle::Active, true, 1, 0, 9).is_err());
        assert!(
            p.authorize(
                SolanaNetwork::Devnet,
                Lifecycle::Active,
                true,
                &address(4),
                1,
                0,
                9
            )
            .is_err()
        );
        let mut locked = p.clone();
        locked.locked = true;
        assert!(
            locked
                .authorize(
                    SolanaNetwork::Devnet,
                    Lifecycle::Active,
                    true,
                    &dest,
                    1,
                    0,
                    9
                )
                .is_err()
        );
    }

    #[test]
    fn confirmation_regressions_slot_disagreement_and_failed_forks_cannot_create_receipts() {
        let (owner, sig, status, tx) = fixture();
        for (field, value) in [
            ("confirmationStatus", json!("processed")),
            ("confirmationStatus", json!("confirmed")),
            ("confirmationStatus", json!(null)),
            ("slot", json!(43)),
            ("err", json!({"InstructionError":[0,"Custom"]})),
        ] {
            let mut observed = status.clone();
            observed[field] = value;
            assert!(
                VerifiedPayment::verify(
                    ChainEvidence::MockRpc,
                    SolanaNetwork::Devnet,
                    &owner,
                    &sig,
                    &observed,
                    &tx
                )
                .is_err()
            );
        }
        let mut failed = tx.clone();
        failed["meta"]["err"] = json!({"InstructionError":[0,"Custom"]});
        assert!(
            VerifiedPayment::verify(
                ChainEvidence::MockRpc,
                SolanaNetwork::Devnet,
                &owner,
                &sig,
                &status,
                &failed
            )
            .is_err()
        );
    }
}
