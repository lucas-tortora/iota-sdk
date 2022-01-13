// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::{
    signing::{mnemonic::IOTA_COIN_TYPE, GenerateAddressMetadata, Network, SignerHandle},
    Client, Error, Result,
};

use bee_message::address::Address;

use std::ops::Range;

/// Generated addresses
#[derive(Debug, Clone)]
pub struct RawAddresses {
    /// Public addresses
    pub public: Vec<Address>,
    /// Internal/change addresses https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki#change
    pub internal: Vec<Address>,
}

/// Generated addresses bech32 encoded
#[derive(Debug, Clone)]
pub struct Bech32Addresses {
    /// Public addresses
    pub public: Vec<String>,
    /// Internal/change addresses https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki#change
    pub internal: Vec<String>,
}

/// Builder of get_addresses API
pub struct GetAddressesBuilder<'a> {
    client: Option<&'a Client>,
    signer: Option<&'a SignerHandle>,
    account_index: u32,
    range: Range<u32>,
    bech32_hrp: Option<String>,
    metadata: GenerateAddressMetadata,
}

impl<'a> Default for GetAddressesBuilder<'a> {
    fn default() -> Self {
        Self {
            client: None,
            signer: None,
            account_index: 0,
            range: 0..super::ADDRESS_GAP_RANGE,
            bech32_hrp: None,
            metadata: GenerateAddressMetadata {
                syncing: true,
                network: Network::Testnet,
            },
        }
    }
}

impl<'a> GetAddressesBuilder<'a> {
    /// Create get_addresses builder
    pub fn new(signer: &'a SignerHandle) -> Self {
        Self {
            signer: Some(signer),
            ..Default::default()
        }
    }

    /// Provide a client to get the bech32_hrp from the node
    pub fn with_client(mut self, client: &'a Client) -> Self {
        self.client.replace(client);
        self
    }

    /// Set the account index
    pub fn with_account_index(mut self, account_index: u32) -> Self {
        self.account_index = account_index;
        self
    }

    /// Set range to the builder
    pub fn with_range(mut self, range: Range<u32>) -> Self {
        self.range = range;
        self
    }

    /// Set bech32 human readable part (hrp)
    pub fn with_bech32_hrp(mut self, bech32_hrp: String) -> Self {
        self.bech32_hrp.replace(bech32_hrp);
        self
    }

    /// Set the metadata for the address generation (used for ledger to display addresses or not)
    pub fn with_generate_metadata(mut self, metadata: GenerateAddressMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Consume the builder and get a vector of public addresses bech32 encoded
    pub async fn finish(self) -> Result<Vec<String>> {
        let bech32_hrp = match self.bech32_hrp.clone() {
            Some(bech32_hrp) => bech32_hrp,
            None => match self.client {
                Some(client) => client.get_bech32_hrp().await?,
                None => "iota".to_string(),
            },
        };
        let signer = self.signer.ok_or(Error::MissingParameter("signer"))?;
        let mut signer = signer.lock().await;
        let addresses = signer
            .generate_addresses(
                IOTA_COIN_TYPE,
                self.account_index,
                self.range,
                false,
                self.metadata.clone(),
            )
            .await?
            .into_iter()
            .map(|a| a.to_bech32(&bech32_hrp))
            .collect();

        Ok(addresses)
    }

    /// Consume the builder and get the vector of public and internal addresses bech32 encoded
    pub async fn get_all(self) -> Result<Bech32Addresses> {
        let bech32_hrp = match self.bech32_hrp.clone() {
            Some(bech32_hrp) => bech32_hrp,
            None => match self.client {
                Some(client) => client.get_bech32_hrp().await?,
                None => "iota".to_string(),
            },
        };
        let addresses = self.get_all_raw().await?;

        Ok(Bech32Addresses {
            public: addresses.public.into_iter().map(|a| a.to_bech32(&bech32_hrp)).collect(),
            internal: addresses
                .internal
                .into_iter()
                .map(|a| a.to_bech32(&bech32_hrp))
                .collect(),
        })
    }

    /// Consume the builder and get the vector of public and internal addresses
    pub async fn get_all_raw(self) -> Result<RawAddresses> {
        let signer = self.signer.ok_or(Error::MissingParameter("signer"))?;
        let mut signer = signer.lock().await;
        let public_addresses = signer
            .generate_addresses(
                IOTA_COIN_TYPE,
                self.account_index,
                self.range.clone(),
                false,
                self.metadata.clone(),
            )
            .await?;

        let internal_addresses = signer
            .generate_addresses(
                IOTA_COIN_TYPE,
                self.account_index,
                self.range,
                true,
                self.metadata.clone(),
            )
            .await?;

        Ok(RawAddresses {
            public: public_addresses,
            internal: internal_addresses,
        })
    }
}

/// Function to find the index and public (false) or internal (true) type of an Bech32 encoded address
pub async fn search_address(
    signer: &SignerHandle,
    bech32_hrp: &str,
    account_index: u32,
    range: Range<u32>,
    address: &Address,
) -> Result<(u32, bool)> {
    let addresses = GetAddressesBuilder::new(signer)
        .with_account_index(account_index)
        .with_range(range.clone())
        .get_all_raw()
        .await?;
    for index in 0..addresses.public.len() {
        if addresses.public[index] == *address {
            return Ok((range.start + index as u32, false));
        }
        if addresses.internal[index] == *address {
            return Ok((range.start + index as u32, true));
        }
    }
    Err(crate::error::Error::InputAddressNotFound(
        address.to_bech32(bech32_hrp),
        format!("{:?}", range),
    ))
}
