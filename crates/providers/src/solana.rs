//! Read-only Solana adapter. No sendTransaction/requestAirdrop/signing RPC exists in this interface.
use async_trait::async_trait;
use serde_json::{Value, json};
use std::time::Duration;
use supa_domain::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RpcMethod {
    Genesis,
    MintAccount,
    Balance,
    TokenAccounts,
    Signatures,
    SignatureStatus,
    Transaction,
    Simulate,
}
impl RpcMethod {
    pub fn name(self) -> &'static str {
        match self {
            Self::Genesis => "getGenesisHash",
            Self::MintAccount => "getAccountInfo",
            Self::Balance => "getBalance",
            Self::TokenAccounts => "getTokenAccountsByOwner",
            Self::Signatures => "getSignaturesForAddress",
            Self::SignatureStatus => "getSignatureStatuses",
            Self::Transaction => "getTransaction",
            Self::Simulate => "simulateTransaction",
        }
    }
}
#[async_trait]
pub trait SolanaRpc: Send + Sync {
    async fn read(&self, method: RpcMethod, params: Value) -> Result<Value>;
    fn evidence(&self) -> ChainEvidence {
        ChainEvidence::MockRpc
    }
}
pub struct HttpSolanaRpc {
    client: reqwest::Client,
    endpoint: reqwest::Url,
}
impl HttpSolanaRpc {
    pub fn new(endpoint: &str, network: SolanaNetwork) -> Result<Self> {
        network.require_devnet()?;
        let url = reqwest::Url::parse(endpoint).map_err(|_| Error::Invalid)?;
        if !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
            || !(url.scheme() == "https"
                || (url.scheme() == "http"
                    && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))))
        {
            return Err(Error::Invalid);
        }
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .build()
            .map_err(|_| Error::Rpc)?;
        Ok(Self {
            client,
            endpoint: url,
        })
    }
    async fn once(&self, method: RpcMethod, params: &Value) -> Result<Value> {
        let mut response = self
            .client
            .post(self.endpoint.clone())
            .json(&json!({"jsonrpc":"2.0","id":1,"method":method.name(),"params":params}))
            .send()
            .await
            .map_err(|_| Error::Rpc)?;
        if !response.status().is_success() {
            return Err(Error::Rpc);
        }
        let mut bytes = vec![];
        while let Some(chunk) = response.chunk().await.map_err(|_| Error::Rpc)? {
            if bytes.len() + chunk.len() > 2_000_000 {
                return Err(Error::Rpc);
            }
            bytes.extend_from_slice(&chunk);
        }
        let v: Value = serde_json::from_slice(&bytes).map_err(|_| Error::Rpc)?;
        if v["id"] != 1 || !v["error"].is_null() || v.get("result").is_none() {
            return Err(Error::Rpc);
        }
        Ok(v["result"].clone())
    }
}
#[async_trait]
impl SolanaRpc for HttpSolanaRpc {
    async fn read(&self, m: RpcMethod, p: Value) -> Result<Value> {
        let first = self.once(m, &p).await;
        if first.is_ok() {
            return first;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
        self.once(m, &p).await
    }
    fn evidence(&self) -> ChainEvidence {
        ChainEvidence::ConfiguredRpc
    }
}
pub fn decode_token_accounts(
    value: &Value,
    owner: &str,
    mint: &str,
    decimals: u8,
) -> Result<Vec<TokenAccountBalance>> {
    validate_solana_address(owner)?;
    validate_solana_address(mint)?;
    let rows = value["value"]
        .as_array()
        .filter(|v| v.len() <= 100)
        .ok_or(Error::Rpc)?;
    let mut result = vec![];
    let mut seen = std::collections::BTreeSet::new();
    for row in rows {
        let address = row["pubkey"].as_str().ok_or(Error::Rpc)?;
        validate_solana_address(address)?;
        let info = &row["account"]["data"]["parsed"]["info"];
        if !seen.insert(address)
            || row["account"]["owner"] != TOKEN_PROGRAM
            || row["account"]["data"]["parsed"]["type"] != "account"
            || info["owner"] != owner
            || info["mint"] != mint
            || info["tokenAmount"]["decimals"] != decimals
        {
            return Err(Error::Rpc);
        }
        let atoms = info["tokenAmount"]["amount"]
            .as_str()
            .and_then(|s| s.parse::<i64>().ok())
            .filter(|n| *n >= 0)
            .ok_or(Error::Rpc)?;
        result.push(TokenAccountBalance {
            address: address.into(),
            mint: mint.into(),
            atoms,
            decimals,
        });
    }
    Ok(result)
}
pub async fn check_network(rpc: &dyn SolanaRpc, network: SolanaNetwork) -> Result<()> {
    network.require_devnet()?;
    if rpc.read(RpcMethod::Genesis, json!([])).await?.as_str() != Some(network.genesis()) {
        return Err(Error::Rpc);
    }
    Ok(())
}
pub async fn read_balances(rpc: &dyn SolanaRpc, wallet: &WalletIdentity) -> Result<WalletBalance> {
    check_network(rpc, wallet.network).await?;
    let sol = rpc
        .read(
            RpcMethod::Balance,
            json!([wallet.address,{"commitment":"finalized"}]),
        )
        .await?;
    let tokens=rpc.read(RpcMethod::TokenAccounts,json!([wallet.address,{"mint":wallet.network.usdc_mint()},{"encoding":"jsonParsed","commitment":"finalized"}])).await?;
    let token_accounts =
        decode_token_accounts(&tokens, &wallet.address, wallet.network.usdc_mint(), 6)?;
    let usdc_atoms = token_accounts
        .iter()
        .try_fold(0i64, |a, b| a.checked_add(b.atoms).ok_or(Error::Rpc))?;
    let slot = sol["context"]["slot"]
        .as_i64()
        .zip(tokens["context"]["slot"].as_i64())
        .map(|(a, b)| a.min(b))
        .filter(|n| *n >= 0)
        .ok_or(Error::Rpc)?;
    Ok(WalletBalance {
        evidence_source: rpc.evidence(),
        network: wallet.network,
        address: wallet.address.clone(),
        slot,
        sol_lamports: sol["value"]
            .as_i64()
            .filter(|n| *n >= 0)
            .ok_or(Error::Rpc)?,
        usdc_atoms,
        token_accounts,
    })
}
pub async fn verify_incoming(
    rpc: &dyn SolanaRpc,
    wallet: &WalletIdentity,
    signature: &str,
) -> Result<VerifiedPayment> {
    validate_solana_signature(signature)?;
    check_network(rpc, wallet.network).await?;
    let status = rpc
        .read(
            RpcMethod::SignatureStatus,
            json!([[signature],{"searchTransactionHistory":true}]),
        )
        .await?;
    let s = status["value"]
        .as_array()
        .filter(|a| a.len() == 1)
        .ok_or(Error::Rpc)?;
    if s[0].is_null() {
        return Err(Error::Unconfirmed);
    }
    let tx=rpc.read(RpcMethod::Transaction,json!([signature,{"encoding":"jsonParsed","commitment":"finalized","maxSupportedTransactionVersion":0}])).await?;
    if tx.is_null() {
        return Err(Error::Unconfirmed);
    }
    VerifiedPayment::verify(
        rpc.evidence(),
        wallet.network,
        &wallet.address,
        signature,
        &s[0],
        &tx,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rpc_configuration_never_enables_mainnet_or_unsafe_transport() {
        assert!(HttpSolanaRpc::new("https://example.org/rpc", SolanaNetwork::Devnet).is_ok());
        for url in [
            "http://example.org",
            "https://user:password@example.org",
            "ftp://localhost",
            "https://example.org/#secret",
        ] {
            assert!(HttpSolanaRpc::new(url, SolanaNetwork::Devnet).is_err());
        }
        assert!(HttpSolanaRpc::new("https://example.org", SolanaNetwork::Mainnet).is_err());
    }
    #[test]
    fn token_decoder_rejects_foreign_accounts_bad_mints_and_floats() {
        let owner = bs58::encode([1; 32]).into_string();
        let token = bs58::encode([2; 32]).into_string();
        let mint = SolanaNetwork::Devnet.usdc_mint();
        let v = json!({"value":[{"pubkey":token,"account":{"owner":TOKEN_PROGRAM,"data":{"parsed":{"type":"account","info":{"owner":owner,"mint":mint,"tokenAmount":{"amount":"1234567","decimals":6}}}}}}]});
        assert_eq!(
            decode_token_accounts(&v, &owner, mint, 6).unwrap()[0].atoms,
            1_234_567
        );
        for (pointer, value) in [
            ("/value/0/account/owner", json!(owner)),
            ("/value/0/account/data/parsed/info/owner", json!(token)),
            (
                "/value/0/account/data/parsed/info/mint",
                json!(SolanaNetwork::Mainnet.usdc_mint()),
            ),
            (
                "/value/0/account/data/parsed/info/tokenAmount/amount",
                json!(1.2),
            ),
            (
                "/value/0/account/data/parsed/info/tokenAmount/amount",
                json!("-1"),
            ),
            (
                "/value/0/account/data/parsed/info/tokenAmount/decimals",
                json!(9),
            ),
        ] {
            let mut bad = v.clone();
            *bad.pointer_mut(pointer).unwrap() = value;
            assert!(decode_token_accounts(&bad, &owner, mint, 6).is_err());
        }
        let mut duplicate = v.clone();
        duplicate["value"]
            .as_array_mut()
            .unwrap()
            .push(v["value"][0].clone());
        assert!(decode_token_accounts(&duplicate, &owner, mint, 6).is_err());
    }
}

/// Operator review only: no signature, broadcast, policy grant, or reservation is produced.
/// A future signer MUST additionally parse the transaction, validate destinations/fees and recheck policy.
pub async fn simulate_for_review(
    rpc: &dyn SolanaRpc,
    network: SolanaNetwork,
    unsigned_base64: &str,
) -> Result<serde_json::Value> {
    use base64::Engine;
    if unsigned_base64.len() > 1644 {
        return Err(Error::Invalid);
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(unsigned_base64)
        .map_err(|_| Error::Invalid)?;
    if bytes.is_empty() || bytes.len() > 1232 {
        return Err(Error::Invalid);
    }
    check_network(rpc, network).await?;
    let result=rpc.read(RpcMethod::Simulate,json!([unsigned_base64,{"encoding":"base64","sigVerify":false,"replaceRecentBlockhash":true,"commitment":"finalized"}])).await?;
    let slot = result["context"]["slot"]
        .as_i64()
        .filter(|n| *n >= 0)
        .ok_or(Error::Rpc)?;
    if result["value"].get("err").is_none() || !result["value"]["err"].is_null() {
        return Err(Error::FinancialPolicy);
    }
    let units = result["value"]["unitsConsumed"]
        .as_u64()
        .filter(|n| *n <= 200_000)
        .ok_or(Error::ResourceBudget)?;
    Ok(
        json!({"slot":slot,"units_consumed":units,"signing_authorized":false,"broadcast_enabled":false}),
    )
}

#[cfg(test)]
mod transport_tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    async fn server(
        responses: Vec<(u64, &'static str)>,
    ) -> (String, tokio::task::JoinHandle<usize>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let mut count = 0;
            for (delay, body) in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = vec![0; 8192];
                let _ = stream.read(&mut request).await.unwrap();
                count += 1;
                tokio::time::sleep(Duration::from_millis(delay)).await;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes()).await;
            }
            count
        });
        (endpoint, task)
    }
    #[tokio::test]
    #[ignore = "requires local loopback sockets; no external RPC"]
    async fn bounded_timeout_retry_recovers_without_exposing_rpc_secrets() {
        let (endpoint, task) = server(vec![
            (
                60,
                r#"{"id":1,"error":{"message":"private-rpc-credential"}}"#,
            ),
            (0, r#"{"id":1,"result":"recovered"}"#),
        ])
        .await;
        let rpc = HttpSolanaRpc {
            client: reqwest::Client::builder()
                .timeout(Duration::from_millis(35))
                .retry(reqwest::retry::never())
                .build()
                .unwrap(),
            endpoint: reqwest::Url::parse(&endpoint).unwrap(),
        };
        assert_eq!(
            rpc.read(RpcMethod::Genesis, json!([])).await.unwrap(),
            "recovered"
        );
        assert_eq!(task.await.unwrap(), 2);
        let (endpoint, task) = server(vec![
            (
                0,
                r#"{"id":1,"error":{"message":"private-rpc-credential"}}"#,
            ),
            (
                0,
                r#"{"id":1,"error":{"message":"private-rpc-credential"}}"#,
            ),
        ])
        .await;
        let rpc = HttpSolanaRpc::new(&endpoint, SolanaNetwork::Devnet).unwrap();
        let error = rpc.read(RpcMethod::Genesis, json!([])).await.unwrap_err();
        assert_eq!(error.to_string(), "rpc_failed");
        assert_eq!(task.await.unwrap(), 2);
    }
    struct Simulation {
        fail: bool,
    }
    #[async_trait]
    impl SolanaRpc for Simulation {
        async fn read(&self, m: RpcMethod, p: Value) -> Result<Value> {
            match m {
                RpcMethod::Genesis => Ok(json!(SolanaNetwork::Devnet.genesis())),
                RpcMethod::Simulate => {
                    assert_eq!(p[1]["sigVerify"], false);
                    assert_eq!(p[1]["replaceRecentBlockhash"], true);
                    Ok(
                        json!({"context":{"slot":42},"value":{"err":if self.fail {json!({"secret":"private"})}else{Value::Null},"unitsConsumed":1000}}),
                    )
                }
                _ => Err(Error::Denied),
            }
        }
    }
    #[tokio::test]
    async fn simulation_is_required_review_evidence_and_never_authorizes_signing() {
        let result =
            simulate_for_review(&Simulation { fail: false }, SolanaNetwork::Devnet, "AA==")
                .await
                .unwrap();
        assert_eq!(result["signing_authorized"], false);
        assert!(matches!(
            simulate_for_review(&Simulation { fail: true }, SolanaNetwork::Devnet, "AA==").await,
            Err(Error::FinancialPolicy)
        ));
        assert!(
            simulate_for_review(&Simulation { fail: false }, SolanaNetwork::Mainnet, "AA==")
                .await
                .is_err()
        );
        assert!(
            simulate_for_review(
                &Simulation { fail: false },
                SolanaNetwork::Devnet,
                "secret$"
            )
            .await
            .is_err()
        );
    }
}

/// Verify the canonical Devnet USDC account on-chain; token symbols/names are never trusted.
pub async fn verify_usdc_mint(
    rpc: &dyn SolanaRpc,
    network: SolanaNetwork,
) -> Result<serde_json::Value> {
    check_network(rpc, network).await?;
    let account = rpc
        .read(
            RpcMethod::MintAccount,
            json!([network.usdc_mint(),{"encoding":"jsonParsed","commitment":"finalized"}]),
        )
        .await?;
    let value = &account["value"];
    let info = &value["data"]["parsed"]["info"];
    if value["owner"] != TOKEN_PROGRAM
        || value["executable"] != false
        || value["data"]["parsed"]["type"] != "mint"
        || info["decimals"] != 6
        || info["isInitialized"] != true
    {
        return Err(Error::InvalidPayment);
    }
    let slot = account["context"]["slot"]
        .as_i64()
        .filter(|n| *n >= 0)
        .ok_or(Error::Rpc)?;
    Ok(
        json!({"network":network,"genesis_hash":network.genesis(),"mint":network.usdc_mint(),"decimals":6,"slot":slot,"confirmation_status":"finalized","evidence_source":rpc.evidence()}),
    )
}

#[cfg(test)]
mod devnet_tests {
    use super::*;
    struct Genesis(&'static str);
    #[async_trait]
    impl SolanaRpc for Genesis {
        async fn read(&self, _: RpcMethod, _: Value) -> Result<Value> {
            Ok(json!(self.0))
        }
    }
    #[tokio::test]
    async fn actual_full_genesis_hash_is_accepted_and_truncated_chain_identifier_is_rejected() {
        // Literal captured from real Devnet and cross-checked against Solana's genesis_config source.
        assert!(
            check_network(
                &Genesis("EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG"),
                SolanaNetwork::Devnet
            )
            .await
            .is_ok()
        );
        assert!(
            check_network(
                &Genesis("EtWTRABZaYq6iMfeYKouRu166VU2xqa1"),
                SolanaNetwork::Devnet
            )
            .await
            .is_err()
        );
        for n in [SolanaNetwork::Devnet, SolanaNetwork::Mainnet] {
            validate_solana_address(n.genesis()).unwrap();
        }
    }
    struct Mint(Value);
    #[async_trait]
    impl SolanaRpc for Mint {
        async fn read(&self, m: RpcMethod, p: Value) -> Result<Value> {
            match m {
                RpcMethod::Genesis => Ok(json!("EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG")),
                RpcMethod::MintAccount => {
                    assert_eq!(p[0], "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU");
                    Ok(self.0.clone())
                }
                _ => Err(Error::Denied),
            }
        }
    }
    #[tokio::test]
    async fn mint_verification_rejects_uninitialized_foreign_program_and_malicious_metadata() {
        let good = json!({"context":{"slot":42},"value":{"owner":TOKEN_PROGRAM,"executable":false,"data":{"parsed":{"type":"mint","info":{"decimals":6,"isInitialized":true}}}}});
        assert!(
            verify_usdc_mint(&Mint(good.clone()), SolanaNetwork::Devnet)
                .await
                .is_ok()
        );
        for (path, bad) in [
            ("/value/owner", json!("malicious-program")),
            ("/value/executable", json!(true)),
            ("/value/data/parsed/type", json!("account")),
            ("/value/data/parsed/info/decimals", json!(9)),
            ("/value/data/parsed/info/isInitialized", json!(false)),
        ] {
            let mut value = good.clone();
            *value.pointer_mut(path).unwrap() = bad;
            assert!(
                verify_usdc_mint(&Mint(value), SolanaNetwork::Devnet)
                    .await
                    .is_err()
            );
        }
        assert!(
            verify_usdc_mint(&Mint(good), SolanaNetwork::Mainnet)
                .await
                .is_err()
        );
    }
}
