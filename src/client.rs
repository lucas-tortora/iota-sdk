// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! The Client module to connect through HORNET or Bee with API usages
#[cfg(feature = "mqtt")]
use crate::node_api::mqtt::{BrokerOptions, MqttEvent, MqttManager, TopicHandlerMap};

use crate::{
    api::{
        miner::{ClientMiner, ClientMinerBuilder},
        *,
    },
    builder::{ClientBuilder, NetworkInfo, GET_API_TIMEOUT},
    error::*,
    node::*,
    node_manager::Node,
    signing::SignerHandle,
    utils::{
        bech32_to_hex, generate_mnemonic, hash_network, hex_public_key_to_bech32_address, hex_to_bech32,
        is_address_valid, mnemonic_to_hex_seed, mnemonic_to_seed, parse_bech32_address,
    },
};

use bee_message::{
    address::Address,
    input::{UtxoInput, INPUT_COUNT_MAX},
    output::OutputId,
    parent::Parents,
    payload::{transaction::TransactionId, Payload},
    Message, MessageBuilder, MessageId,
};
use bee_pow::providers::NonceProviderBuilder;
use bee_rest_api::types::{
    body::SuccessBody,
    dtos::{LedgerInclusionStateDto, PeerDto, ReceiptDto},
    responses::{
        BalanceAddressResponse, InfoResponse as NodeInfo, MilestoneResponse, OutputResponse, TreasuryResponse,
        UtxoChangesResponse as MilestoneUTXOChanges,
    },
};
use crypto::keys::slip10::Seed;
use packable::PackableExt;

use crate::builder::TIPS_INTERVAL;
#[cfg(feature = "mqtt")]
use rumqttc::AsyncClient as MqttClient;
#[cfg(feature = "mqtt")]
use tokio::sync::watch::{Receiver as WatchReceiver, Sender as WatchSender};
#[cfg(not(feature = "wasm"))]
use tokio::{
    runtime::Runtime,
    sync::broadcast::{Receiver, Sender},
    time::{sleep, Duration as TokioDuration},
};
use url::Url;

#[cfg(not(feature = "wasm"))]
use std::collections::HashMap;
use std::{
    collections::HashSet,
    ops::Range,
    str::FromStr,
    sync::{Arc, RwLock},
    time::Duration,
};

/// NodeInfo wrapper which contains the nodeinfo and the url from the node (useful when multiple nodes are used)
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeInfoWrapper {
    /// The returned nodeinfo
    pub nodeinfo: NodeInfo,
    /// The url from the node which returned the nodeinfo
    pub url: String,
}

/// An instance of the client using HORNET or Bee URI
// #[cfg_attr(feature = "wasm", derive(Clone))]
#[derive(Clone)]
pub struct Client {
    #[allow(dead_code)]
    #[cfg(not(feature = "wasm"))]
    pub(crate) runtime: Option<Arc<Runtime>>,
    /// Node manager
    pub(crate) node_manager: crate::node_manager::NodeManager,
    /// Flag to stop the node syncing
    #[cfg(not(feature = "wasm"))]
    pub(crate) sync_kill_sender: Option<Arc<Sender<()>>>,
    /// A MQTT client to subscribe/unsubscribe to topics.
    #[cfg(feature = "mqtt")]
    pub(crate) mqtt_client: Option<MqttClient>,
    #[cfg(feature = "mqtt")]
    pub(crate) mqtt_topic_handlers: Arc<tokio::sync::RwLock<TopicHandlerMap>>,
    #[cfg(feature = "mqtt")]
    pub(crate) broker_options: BrokerOptions,
    #[cfg(feature = "mqtt")]
    pub(crate) mqtt_event_channel: (Arc<WatchSender<MqttEvent>>, WatchReceiver<MqttEvent>),
    pub(crate) network_info: Arc<RwLock<NetworkInfo>>,
    /// HTTP request timeout.
    pub(crate) request_timeout: Duration,
    /// HTTP request timeout for remote PoW API call.
    pub(crate) remote_pow_timeout: Duration,
    #[allow(dead_code)] // not used for wasm
    /// pow_worker_count for local PoW.
    pub(crate) pow_worker_count: Option<usize>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("Client");
        d.field("node_manager", &self.node_manager);
        #[cfg(feature = "mqtt")]
        d.field("broker_options", &self.broker_options);
        d.field("network_info", &self.network_info).finish()
    }
}

impl Drop for Client {
    /// Gracefully shutdown the `Client`
    fn drop(&mut self) {
        #[cfg(not(feature = "wasm"))]
        if let Some(sender) = self.sync_kill_sender.take() {
            sender.send(()).expect("failed to stop syncing process");
        }

        #[cfg(not(feature = "wasm"))]
        if let Some(runtime) = self.runtime.take() {
            if let Ok(runtime) = Arc::try_unwrap(runtime) {
                runtime.shutdown_background()
            }
        }

        #[cfg(feature = "mqtt")]
        if let Some(mqtt_client) = self.mqtt_client.take() {
            std::thread::spawn(move || {
                // ignore errors in case the event loop was already dropped
                // .cancel() finishes the event loop right away
                let _ = crate::async_runtime::block_on(mqtt_client.cancel());
            })
            .join()
            .unwrap();
        }
    }
}

impl Client {
    /// Create the builder to instntiate the IOTA Client.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Sync the node lists per node_sync_interval milliseconds
    #[cfg(not(feature = "wasm"))]
    pub(crate) fn start_sync_process(
        runtime: &Runtime,
        sync: Arc<RwLock<HashSet<Node>>>,
        nodes: HashSet<Node>,
        node_sync_interval: Duration,
        network_info: Arc<RwLock<NetworkInfo>>,
        mut kill: Receiver<()>,
    ) {
        let node_sync_interval =
            TokioDuration::from_nanos(node_sync_interval.as_nanos().try_into().unwrap_or(TIPS_INTERVAL));

        runtime.spawn(async move {
            loop {
                tokio::select! {
                    _ = async {
                            // delay first since the first `sync_nodes` call is made by the builder
                            // to ensure the node list is filled before the client is used
                            sleep(node_sync_interval).await;
                            Client::sync_nodes(&sync, &nodes, &network_info).await;
                    } => {}
                    _ = kill.recv() => {}
                }
            }
        });
    }

    #[cfg(not(feature = "wasm"))]
    pub(crate) async fn sync_nodes(
        sync: &Arc<RwLock<HashSet<Node>>>,
        nodes: &HashSet<Node>,
        network_info: &Arc<RwLock<NetworkInfo>>,
    ) {
        let mut synced_nodes = HashSet::new();
        let mut network_nodes: HashMap<String, Vec<(NodeInfo, Node)>> = HashMap::new();
        for node in nodes {
            // Put the healthy node url into the network_nodes
            if let Ok(info) = Client::get_node_info(&node.url.to_string(), None, None).await {
                if info.is_healthy {
                    match network_nodes.get_mut(&info.network_id) {
                        Some(network_id_entry) => {
                            network_id_entry.push((info, node.clone()));
                        }
                        None => match &network_info
                            .read()
                            .map_or(NetworkInfo::default().network, |info| info.network.clone())
                        {
                            Some(id) => {
                                if info.network_id.contains(id) {
                                    network_nodes.insert(info.network_id.clone(), vec![(info, node.clone())]);
                                }
                            }
                            None => {
                                network_nodes.insert(info.network_id.clone(), vec![(info, node.clone())]);
                            }
                        },
                    }
                }
            }
        }
        // Get network_id with the most nodes
        let mut most_nodes = ("network_id", 0);
        for (network_id, node) in network_nodes.iter() {
            if node.len() > most_nodes.1 {
                most_nodes.0 = network_id;
                most_nodes.1 = node.len();
            }
        }
        if let Some(nodes) = network_nodes.get(most_nodes.0) {
            for (info, node_url) in nodes.iter() {
                if let Ok(mut client_network_info) = network_info.write() {
                    client_network_info.network_id = hash_network(&info.network_id).ok();
                    client_network_info.min_pow_score = info.min_pow_score;
                    client_network_info.bech32_hrp = info.bech32_hrp.clone();
                    if !client_network_info.local_pow {
                        if info.features.contains(&"PoW".to_string()) {
                            synced_nodes.insert(node_url.clone());
                        }
                    } else {
                        synced_nodes.insert(node_url.clone());
                    }
                }
            }
        }

        // Update the sync list
        if let Ok(mut sync) = sync.write() {
            *sync = synced_nodes;
        }
    }

    /// Get a node candidate from the synced node pool.
    pub async fn get_node(&self) -> Result<Node> {
        if let Some(primary_node) = &self.node_manager.primary_node {
            return Ok(primary_node.clone());
        }
        let pool = self.node_manager.nodes.clone();
        Ok(pool.into_iter().next().ok_or(Error::SyncedNodePoolEmpty)?)
    }

    /// Gets the network id of the node we're connecting to.
    pub async fn get_network_id(&self) -> Result<u64> {
        let network_info = self.get_network_info().await?;
        network_info
            .network_id
            .ok_or(Error::MissingParameter("Missing network id."))
    }

    /// Gets the miner to use based on the PoW setting
    pub async fn get_pow_provider(&self) -> ClientMiner {
        ClientMinerBuilder::new()
            .with_local_pow(self.get_local_pow().await)
            .finish()
    }

    /// Gets the network related information such as network_id and min_pow_score
    /// and if it's the default one, sync it first.
    pub async fn get_network_info(&self) -> Result<NetworkInfo> {
        let not_synced = self.network_info.read().map_or(true, |info| info.network_id.is_none());

        if not_synced {
            let info = self.get_info().await?.nodeinfo;
            let network_id = hash_network(&info.network_id).ok();
            {
                let mut client_network_info = self.network_info.write().map_err(|_| crate::Error::PoisonError)?;
                client_network_info.network_id = network_id;
                client_network_info.min_pow_score = info.min_pow_score;
                client_network_info.bech32_hrp = info.bech32_hrp;
            }
        }
        let res = self
            .network_info
            .read()
            .map_or(NetworkInfo::default(), |info| info.clone());
        Ok(res)
    }

    /// returns the bech32_hrp
    pub async fn get_bech32_hrp(&self) -> Result<String> {
        Ok(self.get_network_info().await?.bech32_hrp)
    }

    /// returns the min pow score
    pub async fn get_min_pow_score(&self) -> Result<f64> {
        Ok(self.get_network_info().await?.min_pow_score)
    }

    /// returns the tips interval
    pub async fn get_tips_interval(&self) -> u64 {
        self.network_info
            .read()
            .map_or(TIPS_INTERVAL, |info| info.tips_interval)
    }

    /// returns the local pow
    pub async fn get_local_pow(&self) -> bool {
        self.network_info
            .read()
            .map_or(NetworkInfo::default().local_pow, |info| info.local_pow)
    }

    /// returns the unsynced nodes.
    #[cfg(not(feature = "wasm"))]
    pub async fn unsynced_nodes(&self) -> HashSet<&Node> {
        self.node_manager.synced_nodes.read().map_or(HashSet::new(), |synced| {
            self.node_manager
                .nodes
                .iter()
                .filter(|node| !synced.contains(node))
                .collect()
        })
    }

    /// Function to find inputs from addresses for a provided amount (useful for offline signing)
    pub async fn find_inputs(&self, addresses: Vec<String>, amount: u64) -> Result<Vec<UtxoInput>> {
        // Get outputs from node and select inputs
        let mut available_outputs = Vec::new();
        for address in addresses {
            available_outputs.extend_from_slice(
                &self
                    .get_address()
                    .output_ids(OutputsOptions {
                        bech32_address: Some(address.to_string()),
                    })
                    .await?,
            );
        }

        let mut extended_outputs = Vec::new();

        for output_id in available_outputs.into_iter() {
            let utxo_input = UtxoInput::from(output_id);
            let output_data = self.get_output(utxo_input.output_id()).await?;
            let (amount, _) = ClientMessageBuilder::get_output_amount_and_address(&output_data.output)?;
            extended_outputs.push((utxo_input, amount));
        }
        extended_outputs.sort_by(|l, r| r.1.cmp(&l.1));

        let mut total_already_spent = 0;
        let mut selected_inputs = Vec::new();
        for (_offset, output_wrapper) in extended_outputs
            .into_iter()
            // Max inputs is 127
            .take(INPUT_COUNT_MAX.into())
            .enumerate()
        {
            // Break if we have enough funds and don't create dust for the remainder
            if total_already_spent == amount || total_already_spent >= amount {
                break;
            }
            selected_inputs.push(output_wrapper.0.clone());
            total_already_spent += output_wrapper.1;
        }

        if total_already_spent < amount {
            return Err(crate::Error::NotEnoughBalance(total_already_spent, amount));
        }

        Ok(selected_inputs)
    }

    ///////////////////////////////////////////////////////////////////////
    // MQTT API
    //////////////////////////////////////////////////////////////////////

    /// Returns a handle to the MQTT topics manager.
    #[cfg(feature = "mqtt")]
    pub fn subscriber(&mut self) -> MqttManager<'_> {
        MqttManager::new(self)
    }

    /// Returns the mqtt event receiver.
    #[cfg(feature = "mqtt")]
    pub fn mqtt_event_receiver(&self) -> WatchReceiver<MqttEvent> {
        self.mqtt_event_channel.1.clone()
    }

    //////////////////////////////////////////////////////////////////////
    // Node API
    //////////////////////////////////////////////////////////////////////

    pub(crate) fn get_timeout(&self) -> Duration {
        self.request_timeout
    }
    pub(crate) fn get_remote_pow_timeout(&self) -> Duration {
        self.remote_pow_timeout
    }

    /// GET /health endpoint
    pub async fn get_node_health(url: &str) -> Result<bool> {
        let mut url = Url::parse(url)?;
        url.set_path("health");
        let status = crate::node_manager::HttpClient::new()
            .get(Node { url, jwt: None }, GET_API_TIMEOUT)
            .await?
            .status();
        match status {
            200 => Ok(true),
            _ => Ok(false),
        }
    }

    /// GET /health endpoint
    pub async fn get_health(&self) -> Result<bool> {
        let mut node = self.get_node().await?;
        node.url.set_path("health");
        let status = self.node_manager.http_client.get(node, GET_API_TIMEOUT).await?.status();
        match status {
            200 => Ok(true),
            _ => Ok(false),
        }
    }

    // todo: only used during syncing, can it be replaced with the other node info function?
    /// GET /api/v2/info endpoint
    pub async fn get_node_info(
        url: &str,
        jwt: Option<String>,
        auth_name_pwd: Option<(&str, &str)>,
    ) -> Result<NodeInfo> {
        let mut url = crate::node_manager::validate_url(Url::parse(url)?)?;
        if let Some((name, password)) = auth_name_pwd {
            url.set_username(name)
                .map_err(|_| crate::Error::UrlAuthError("username".to_string()))?;
            url.set_password(Some(password))
                .map_err(|_| crate::Error::UrlAuthError("password".to_string()))?;
        }

        let path = "api/v2/info";
        url.set_path(path);

        let resp: SuccessBody<NodeInfo> = crate::node_manager::HttpClient::new()
            .get(Node { url, jwt }, GET_API_TIMEOUT)
            .await?
            .json()
            .await?;

        Ok(resp.data)
    }

    /// Returns the node information together with the url of the used node
    /// GET /api/v2/info endpoint
    pub async fn get_info(&self) -> Result<NodeInfoWrapper> {
        crate::node_api::core_api::routes::get_info(self).await
    }

    /// GET /api/v2/peers endpoint
    pub async fn get_peers(&self) -> Result<Vec<PeerDto>> {
        crate::node_api::core_api::routes::get_peers(self).await
    }

    /// GET /api/v2/tips endpoint
    pub async fn get_tips(&self) -> Result<Vec<MessageId>> {
        crate::node_api::core_api::routes::get_tips(self).await
    }

    /// POST /api/v2/messages endpoint
    pub async fn post_message(&self, message: &Message) -> Result<MessageId> {
        crate::node_api::core_api::routes::post_message(self, message).await
    }

    /// POST JSON to /api/v2/messages endpoint
    pub async fn post_message_json(&self, message: &Message) -> Result<MessageId> {
        crate::node_api::core_api::routes::post_message_json(self, message).await
    }

    /// GET /api/v2/messages/{messageId} endpoint
    pub fn get_message(&self) -> GetMessageBuilder<'_> {
        GetMessageBuilder::new(self)
    }

    /// GET /api/v2/outputs/{outputId} endpoint
    /// Find an output by its transaction_id and corresponding output_index.
    pub async fn get_output(&self, output_id: &OutputId) -> Result<OutputResponse> {
        crate::node_api::core_api::routes::get_output(self, output_id).await
    }

    /// Find all outputs based on the requests criteria. This method will try to query multiple nodes if
    /// the request amount exceeds individual node limit.
    pub async fn find_outputs(&self, outputs: &[UtxoInput], addresses: &[String]) -> Result<Vec<OutputResponse>> {
        let mut output_metadata = Vec::<OutputResponse>::new();
        // Use a `HashSet` to prevent duplicate output.
        let mut output_to_query = HashSet::<UtxoInput>::new();

        // Collect the `UtxoInput` in the HashSet.
        for output in outputs {
            output_to_query.insert(output.to_owned());
        }

        // Use `get_address()` API to get the address outputs first,
        // then collect the `UtxoInput` in the HashSet.
        for address in addresses {
            let address_outputs = self
                .get_address()
                .outputs(OutputsOptions {
                    bech32_address: Some(address.to_string()),
                })
                .await?;
            for output in address_outputs.iter() {
                output_to_query.insert(UtxoInput::from(OutputId::new(
                    TransactionId::from_str(&output.transaction_id)?,
                    output.output_index,
                )?));
            }
        }

        // Use `get_output` API to get the `OutputMetadata`.
        for output in output_to_query {
            let meta_data = self.get_output(output.output_id()).await?;
            output_metadata.push(meta_data);
        }
        Ok(output_metadata)
    }

    /// GET /api/plugins/indexer/v1/outputs{query} endpoint
    pub fn get_address(&self) -> GetAddressBuilder<'_> {
        GetAddressBuilder::new(self)
    }

    /// GET /api/v2/milestones/{index} endpoint
    /// Get the milestone by the given index.
    pub async fn get_milestone(&self, index: u32) -> Result<MilestoneResponse> {
        crate::node_api::core_api::routes::get_milestone(self, index).await
    }

    /// GET /api/v2/milestones/{index}/utxo-changes endpoint
    /// Get the milestone by the given index.
    pub async fn get_milestone_utxo_changes(&self, index: u32) -> Result<MilestoneUTXOChanges> {
        crate::node_api::core_api::routes::get_milestone_utxo_changes(self, index).await
    }

    /// GET /api/v2/receipts endpoint
    /// Get all receipts.
    pub async fn get_receipts(&self) -> Result<Vec<ReceiptDto>> {
        crate::node_api::core_api::routes::get_receipts(self).await
    }

    /// GET /api/v2/receipts/{migratedAt} endpoint
    /// Get the receipts by the given milestone index.
    pub async fn get_receipts_migrated_at(&self, milestone_index: u32) -> Result<Vec<ReceiptDto>> {
        crate::node_api::core_api::routes::get_receipts_migrated_at(self, milestone_index).await
    }

    /// GET /api/v2/treasury endpoint
    /// Get the treasury output.
    pub async fn get_treasury(&self) -> Result<TreasuryResponse> {
        crate::node_api::core_api::routes::get_treasury(self).await
    }

    /// GET /api/v2/transactions/{transactionId}/included-message
    /// Returns the included message of the transaction.
    pub async fn get_included_message(&self, transaction_id: &TransactionId) -> Result<Message> {
        crate::node_api::core_api::routes::get_included_message(self, transaction_id).await
    }

    /// Reattaches messages for provided message id. Messages can be reattached only if they are valid and haven't been
    /// confirmed for a while.
    pub async fn reattach(&self, message_id: &MessageId) -> Result<(MessageId, Message)> {
        let metadata = self.get_message().metadata(message_id).await?;
        if metadata.should_reattach.unwrap_or(false) {
            self.reattach_unchecked(message_id).await
        } else {
            Err(Error::NoNeedPromoteOrReattach(message_id.to_string()))
        }
    }

    /// Reattach a message without checking if it should be reattached
    pub async fn reattach_unchecked(&self, message_id: &MessageId) -> Result<(MessageId, Message)> {
        // Get the Message object by the MessageID.
        let message = self.get_message().data(message_id).await?;
        let reattach_message = {
            #[cfg(feature = "wasm")]
            {
                let network_id = self.get_network_id().await?;
                let mut tips = self.get_tips().await?;
                tips.sort_unstable_by_key(|a| a.pack_to_vec());
                tips.dedup();
                let mut message_builder = MessageBuilder::<ClientMiner>::new()
                    .with_network_id(network_id)
                    .with_parents(Parents::new(tips)?);
                if let Some(p) = message.payload().to_owned() {
                    message_builder = message_builder.with_payload(p.clone())
                }
                message_builder.finish().map_err(Error::MessageError)?
            }
            #[cfg(not(feature = "wasm"))]
            {
                finish_pow(self, message.payload().cloned()).await?
            }
        };

        // Post the modified
        let message_id = self.post_message(&reattach_message).await?;
        // Get message if we use remote PoW, because the node will change parents and nonce
        let msg = match self.get_local_pow().await {
            true => reattach_message,
            false => self.get_message().data(&message_id).await?,
        };
        Ok((message_id, msg))
    }

    /// Promotes a message. The method should validate if a promotion is necessary through get_message. If not, the
    /// method should error out and should not allow unnecessary promotions.
    pub async fn promote(&self, message_id: &MessageId) -> Result<(MessageId, Message)> {
        let metadata = self.get_message().metadata(message_id).await?;
        if metadata.should_promote.unwrap_or(false) {
            self.promote_unchecked(message_id).await
        } else {
            Err(Error::NoNeedPromoteOrReattach(message_id.to_string()))
        }
    }

    /// Promote a message without checking if it should be promoted
    pub async fn promote_unchecked(&self, message_id: &MessageId) -> Result<(MessageId, Message)> {
        // Create a new message (zero value message) for which one tip would be the actual message
        let mut tips = self.get_tips().await?;
        let min_pow_score = self.get_min_pow_score().await?;
        let network_id = self.get_network_id().await?;
        tips.push(*message_id);
        // Sort tips/parents
        tips.sort_unstable_by_key(|a| a.pack_to_vec());
        tips.dedup();

        let promote_message = MessageBuilder::<ClientMiner>::new()
            .with_network_id(network_id)
            .with_parents(Parents::new(tips)?)
            .with_nonce_provider(self.get_pow_provider().await, min_pow_score)
            .finish()
            .map_err(|_| Error::TransactionError)?;

        let message_id = self.post_message(&promote_message).await?;
        // Get message if we use remote PoW, because the node will change parents and nonce
        let msg = match self.get_local_pow().await {
            true => promote_message,
            false => self.get_message().data(&message_id).await?,
        };
        Ok((message_id, msg))
    }

    //////////////////////////////////////////////////////////////////////
    // High level API
    //////////////////////////////////////////////////////////////////////

    /// A generic send function for easily sending transaction or tagged data messages.
    pub fn message(&self) -> ClientMessageBuilder<'_> {
        ClientMessageBuilder::new(self)
    }

    /// Return a valid unspent address.
    pub fn get_unspent_address<'a>(&'a self, signer: &'a SignerHandle) -> GetUnspentAddressBuilder<'a> {
        GetUnspentAddressBuilder::new(self, signer)
    }

    /// Return a list of addresses from the signer regardless of their validity.
    pub fn get_addresses<'a>(&'a self, signer: &'a SignerHandle) -> GetAddressesBuilder<'a> {
        GetAddressesBuilder::new(signer).with_client(self)
    }

    /// Find all messages by provided message IDs.
    pub async fn find_messages<I: AsRef<[u8]>>(&self, message_ids: &[MessageId]) -> Result<Vec<Message>> {
        let mut messages = Vec::new();

        // Use a `HashSet` to prevent duplicate message_ids.
        let mut message_ids_to_query = HashSet::<MessageId>::new();

        // Collect the `MessageId` in the HashSet.
        for message_id in message_ids {
            message_ids_to_query.insert(message_id.to_owned());
        }

        // Use `get_message().data()` API to get the `Message`.
        for message_id in message_ids_to_query {
            let message = self.get_message().data(&message_id).await?;
            messages.push(message);
        }
        Ok(messages)
    }

    /// Return the balance for a provided signer and its wallet chain account index.
    /// Addresses with balance must be consecutive, so this method will return once it encounters a zero
    /// balance address.
    pub fn get_balance<'a>(&'a self, signer: &'a SignerHandle) -> GetBalanceBuilder<'a> {
        GetBalanceBuilder::new(self, signer)
    }

    /// Return the balance in iota for the given addresses; No seed needed to do this since we are only checking and
    /// already know the addresses.
    pub async fn get_address_balances(&self, addresses: &[String]) -> Result<Vec<BalanceAddressResponse>> {
        let mut address_balance_pairs = Vec::new();
        for address in addresses {
            let balance_response = self.get_address().balance(address).await?;
            address_balance_pairs.push(balance_response);
        }
        Ok(address_balance_pairs)
    }

    /// Retries (promotes or reattaches) a message for provided message id. Message should only be
    /// retried only if they are valid and haven't been confirmed for a while.
    pub async fn retry(&self, message_id: &MessageId) -> Result<(MessageId, Message)> {
        // Get the metadata to check if it needs to promote or reattach
        let message_metadata = self.get_message().metadata(message_id).await?;
        if message_metadata.should_promote.unwrap_or(false) {
            self.promote_unchecked(message_id).await
        } else if message_metadata.should_reattach.unwrap_or(false) {
            self.reattach_unchecked(message_id).await
        } else {
            Err(Error::NoNeedPromoteOrReattach(message_id.to_string()))
        }
    }

    /// Retries (promotes or reattaches) a message for provided message id until it's included (referenced by a
    /// milestone). Default interval is 5 seconds and max attempts is 40. Returns the included message at first position
    /// and additional reattached messages
    pub async fn retry_until_included(
        &self,
        message_id: &MessageId,
        interval: Option<u64>,
        max_attempts: Option<u64>,
    ) -> Result<Vec<(MessageId, Message)>> {
        // Attachments of the Message to check inclusion state
        let mut message_ids = vec![*message_id];
        // Reattached Messages that get returned
        let mut messages_with_id = Vec::new();
        for _ in 0..max_attempts.unwrap_or(40) {
            #[cfg(feature = "wasm")]
            {
                use wasm_timer::Delay;
                Delay::new(Duration::from_secs(interval.unwrap_or(5))).await?;
            }
            #[cfg(not(feature = "wasm"))]
            sleep(Duration::from_secs(interval.unwrap_or(5))).await;
            // Check inclusion state for each attachment
            let message_ids_len = message_ids.len();
            let mut conflicting = false;
            for (index, msg_id) in message_ids.clone().iter().enumerate() {
                let message_metadata = self.get_message().metadata(msg_id).await?;
                if let Some(inclusion_state) = message_metadata.ledger_inclusion_state {
                    match inclusion_state {
                        LedgerInclusionStateDto::Included | LedgerInclusionStateDto::NoTransaction => {
                            // if original message, request it so we can return it on first position
                            if message_id == msg_id {
                                let mut included_and_reattached_messages =
                                    vec![(*message_id, self.get_message().data(message_id).await?)];
                                included_and_reattached_messages.extend(messages_with_id);
                                return Ok(included_and_reattached_messages);
                            } else {
                                // Move included message to first position
                                messages_with_id.rotate_left(index);
                                return Ok(messages_with_id);
                            }
                        }
                        // only set it as conflicting here and don't return, because another reattached message could
                        // have the included transaction
                        LedgerInclusionStateDto::Conflicting => conflicting = true,
                    };
                }
                // Only reattach or promote latest attachment of the message
                if index == message_ids_len - 1 {
                    if message_metadata.should_promote.unwrap_or(false) {
                        // Safe to unwrap since we iterate over it
                        self.promote_unchecked(message_ids.last().unwrap()).await?;
                    } else if message_metadata.should_reattach.unwrap_or(false) {
                        // Safe to unwrap since we iterate over it
                        let reattached = self.reattach_unchecked(message_ids.last().unwrap()).await?;
                        message_ids.push(reattached.0);
                        messages_with_id.push(reattached);
                    }
                }
            }
            // After we checked all our reattached messages, check if the transaction got reattached in another message
            // and confirmed
            if conflicting {
                let message = self.get_message().data(message_id).await?;
                if let Some(Payload::Transaction(transaction_payload)) = message.payload() {
                    let included_message = self.get_included_message(&transaction_payload.id()).await?;
                    let mut included_and_reattached_messages = vec![(included_message.id(), included_message)];
                    included_and_reattached_messages.extend(messages_with_id);
                    return Ok(included_and_reattached_messages);
                }
            }
        }
        Err(Error::TangleInclusionError(message_id.to_string()))
    }

    /// Function to consolidate all funds from a range of addresses to the address with the lowest index in that range
    /// Returns the address to which the funds got consolidated, if any were available
    pub async fn consolidate_funds(
        &self,
        signer: &SignerHandle,
        account_index: u32,
        address_range: Range<u32>,
    ) -> crate::Result<String> {
        crate::api::consolidate_funds(self, signer, account_index, address_range).await
    }

    //////////////////////////////////////////////////////////////////////
    // Utils
    //////////////////////////////////////////////////////////////////////

    /// Transforms bech32 to hex
    pub fn bech32_to_hex(bech32: &str) -> crate::Result<String> {
        bech32_to_hex(bech32)
    }

    /// Transforms a hex encoded address to a bech32 encoded address
    pub async fn hex_to_bech32(&self, hex: &str, bech32_hrp: Option<&str>) -> crate::Result<String> {
        let bech32_hrp = match bech32_hrp {
            Some(hrp) => hrp.into(),
            None => self.get_bech32_hrp().await?,
        };
        hex_to_bech32(hex, &bech32_hrp)
    }

    /// Transforms a hex encoded public key to a bech32 encoded address
    pub async fn hex_public_key_to_bech32_address(&self, hex: &str, bech32_hrp: Option<&str>) -> crate::Result<String> {
        let bech32_hrp = match bech32_hrp {
            Some(hrp) => hrp.into(),
            None => self.get_bech32_hrp().await?,
        };
        hex_public_key_to_bech32_address(hex, &bech32_hrp)
    }

    /// Returns a valid Address parsed from a String.
    pub fn parse_bech32_address(address: &str) -> crate::Result<Address> {
        parse_bech32_address(address)
    }

    /// Checks if a String is a valid bech32 encoded address.
    pub fn is_address_valid(address: &str) -> bool {
        is_address_valid(address)
    }

    /// Generates a new mnemonic.
    pub fn generate_mnemonic() -> Result<String> {
        generate_mnemonic()
    }

    /// Returns a seed for a mnemonic.
    pub fn mnemonic_to_seed(mnemonic: &str) -> Result<Seed> {
        mnemonic_to_seed(mnemonic)
    }

    /// Returns a hex encoded seed for a mnemonic.
    pub fn mnemonic_to_hex_seed(mnemonic: &str) -> Result<String> {
        mnemonic_to_hex_seed(mnemonic)
    }
}
