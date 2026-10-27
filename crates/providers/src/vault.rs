//! Local encrypted custody. No signing, secret export, Debug or model-facing secret API exists.
use ring::{
    aead,
    rand::{SecureRandom, SystemRandom},
    signature::{Ed25519KeyPair, KeyPair},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use supa_domain::*;
use uuid::Uuid;
use zeroize::Zeroizing;
#[derive(Serialize, Deserialize)]
struct Envelope {
    version: u8,
    wallet_id: Uuid,
    network: SolanaNetwork,
    address: String,
    nonce: [u8; 12],
    ciphertext: Vec<u8>,
}
pub struct KeyVault {
    root: PathBuf,
    master: Zeroizing<[u8; 32]>,
}
fn private_file(path: &Path, max: u64) -> Result<File> {
    let meta = std::fs::symlink_metadata(path).map_err(|_| Error::KeyVault)?;
    if !meta.is_file() || meta.len() > max {
        return Err(Error::KeyVault);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o077 != 0 {
            return Err(Error::KeyVault);
        }
    }
    File::open(path).map_err(|_| Error::KeyVault)
}
impl KeyVault {
    pub fn from_files(root: &Path, master_file: &Path) -> Result<Self> {
        let mut bytes = Zeroizing::new(vec![]);
        private_file(master_file, 32)?
            .read_to_end(&mut bytes)
            .map_err(|_| Error::KeyVault)?;
        let key: [u8; 32] = bytes.as_slice().try_into().map_err(|_| Error::KeyVault)?;
        Self::open(root, key)
    }
    pub fn open(root: &Path, master: [u8; 32]) -> Result<Self> {
        if !root.exists() {
            let mut builder = std::fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder.create(root).map_err(|_| Error::KeyVault)?;
        }
        let metadata = std::fs::symlink_metadata(root).map_err(|_| Error::KeyVault)?;
        if !metadata.is_dir() {
            return Err(Error::KeyVault);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(Error::KeyVault);
            }
        }
        Ok(Self {
            root: root.into(),
            master: Zeroizing::new(master),
        })
    }
    fn cipher(&self) -> Result<aead::LessSafeKey> {
        Ok(aead::LessSafeKey::new(
            aead::UnboundKey::new(&aead::CHACHA20_POLY1305, self.master.as_ref())
                .map_err(|_| Error::KeyVault)?,
        ))
    }
    fn aad(id: Uuid, network: SolanaNetwork, address: &str) -> Vec<u8> {
        format!("supa-wallet-v1:{id}:{}:{address}", network.name()).into_bytes()
    }
    pub fn create_public(&self, id: Uuid, network: SolanaNetwork) -> Result<String> {
        network.require_devnet()?;
        let rng = SystemRandom::new();
        let document = Ed25519KeyPair::generate_pkcs8(&rng).map_err(|_| Error::KeyVault)?;
        let key = Ed25519KeyPair::from_pkcs8(document.as_ref()).map_err(|_| Error::KeyVault)?;
        let address = bs58::encode(key.public_key().as_ref()).into_string();
        let mut nonce = [0; 12];
        rng.fill(&mut nonce).map_err(|_| Error::KeyVault)?;
        let mut data = Zeroizing::new(document.as_ref().to_vec());
        self.cipher()?
            .seal_in_place_append_tag(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(Self::aad(id, network, &address)),
                &mut *data,
            )
            .map_err(|_| Error::KeyVault)?;
        let envelope = Envelope {
            version: 1,
            wallet_id: id,
            network,
            address: address.clone(),
            nonce,
            ciphertext: data.to_vec(),
        };
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(self.root.join(format!("{id}.json")))
            .map_err(|_| Error::KeyVault)?;
        file.write_all(&serde_json::to_vec(&envelope).map_err(|_| Error::KeyVault)?)
            .and_then(|()| file.sync_all())
            .map_err(|_| Error::KeyVault)?;
        File::open(&self.root)
            .and_then(|d| d.sync_all())
            .map_err(|_| Error::KeyVault)?;
        Ok(address)
    }
    pub fn recover_public(&self, id: Uuid, network: SolanaNetwork) -> Result<String> {
        let mut bytes = vec![];
        private_file(&self.root.join(format!("{id}.json")), 8192)?
            .read_to_end(&mut bytes)
            .map_err(|_| Error::KeyVault)?;
        let mut envelope: Envelope = serde_json::from_slice(&bytes).map_err(|_| Error::KeyVault)?;
        if envelope.version != 1 || envelope.wallet_id != id || envelope.network != network {
            return Err(Error::KeyVault);
        }
        let mut ciphertext = Zeroizing::new(std::mem::take(&mut envelope.ciphertext));
        let plaintext = self
            .cipher()?
            .open_in_place(
                aead::Nonce::assume_unique_for_key(envelope.nonce),
                aead::Aad::from(Self::aad(id, network, &envelope.address)),
                &mut ciphertext,
            )
            .map_err(|_| Error::KeyVault)?;
        let key = Ed25519KeyPair::from_pkcs8(plaintext).map_err(|_| Error::KeyVault)?;
        let public = bs58::encode(key.public_key().as_ref()).into_string();
        if public != envelope.address {
            return Err(Error::KeyVault);
        }
        Ok(public)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encrypted_wallet_recovery_tampering_wrong_master_and_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("vault");
        let id = Uuid::new_v4();
        let vault = KeyVault::open(&root, [7; 32]).unwrap();
        let public = vault.create_public(id, SolanaNetwork::Devnet).unwrap();
        validate_solana_address(&public).unwrap();
        assert_eq!(
            KeyVault::open(&root, [7; 32])
                .unwrap()
                .recover_public(id, SolanaNetwork::Devnet)
                .unwrap(),
            public
        );
        assert!(
            KeyVault::open(&root, [8; 32])
                .unwrap()
                .recover_public(id, SolanaNetwork::Devnet)
                .is_err()
        );
        assert!(vault.recover_public(id, SolanaNetwork::Mainnet).is_err());
        assert!(
            vault
                .create_public(Uuid::new_v4(), SolanaNetwork::Mainnet)
                .is_err()
        );
        let path = root.join(format!("{id}.json"));
        let bytes = std::fs::read(&path).unwrap();
        let mut envelope: Envelope = serde_json::from_slice(&bytes).unwrap();
        envelope.ciphertext[0] ^= 1;
        std::fs::write(&path, serde_json::to_vec(&envelope).unwrap()).unwrap();
        assert!(vault.recover_public(id, SolanaNetwork::Devnet).is_err());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
            assert!(vault.recover_public(id, SolanaNetwork::Devnet).is_err());
        }
    }
}
