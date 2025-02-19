// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import type { WalletMethodHandler } from './wallet-method-handler';
import {
    Balance,
    AccountMetadata,
    SyncOptions,
    AccountMeta,
    AccountAddress,
    SendParams,
    SendNativeTokensParams,
    SendNftParams,
    AddressWithUnspentOutputs,
    AliasOutputParams,
    FilterOptions,
    GenerateAddressOptions,
    CreateNativeTokenParams,
    MintNftParams,
    OutputData,
    OutputParams,
    OutputsToClaim,
    Transaction,
    TransactionOptions,
    ParticipationOverview,
    ParticipationEventId,
    ParticipationEventStatus,
    ParticipationEventType,
    ParticipationEventWithNodes,
    ParticipationEventRegistrationOptions,
    ParticipationEventMap,
    BuildAliasOutputData,
    BuildBasicOutputData,
    BuildFoundryOutputData,
    BuildNftOutputData,
    SignedTransactionEssence,
    PreparedTransaction,
    PreparedCreateNativeTokenTransactionData,
    ConsolidationParams,
} from '../types/wallet';
import { INode, Burn, PreparedTransactionData } from '../client';
import {
    AliasOutput,
    NftOutput,
    Output,
    BasicOutput,
    FoundryOutput,
    Response,
    PreparedCreateNativeTokenTransaction,
} from '../types';
import { plainToInstance } from 'class-transformer';
import { bigIntToHex, hexToBigInt } from '../types/utils/hex-encoding';

/** The Account class. */
export class Account {
    // private because the data isn't updated
    private meta: AccountMeta;
    private methodHandler: WalletMethodHandler;

    /**
     * @param accountMeta An instance of `AccountMeta`.
     * @param methodHandler A instance of `WalletMethodHandler`.
     */
    constructor(accountMeta: AccountMeta, methodHandler: WalletMethodHandler) {
        this.meta = accountMeta;
        this.methodHandler = methodHandler;
    }

    /** @deprecated use Client::buildAliasOutput() instead. */
    async buildAliasOutput(data: BuildAliasOutputData): Promise<AliasOutput> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'buildAliasOutput',
                data,
            },
        );
        return JSON.parse(response).payload;
    }

    /** @deprecated use Client::buildBasicOutput() instead. */
    async buildBasicOutput(data: BuildBasicOutputData): Promise<BasicOutput> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'buildBasicOutput',
                data,
            },
        );
        return Output.parse(JSON.parse(response).payload) as BasicOutput;
    }

    /** @deprecated use Client::buildFoundryOutput() instead. */
    async buildFoundryOutput(
        data: BuildFoundryOutputData,
    ): Promise<FoundryOutput> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'buildFoundryOutput',
                data,
            },
        );
        return Output.parse(JSON.parse(response).payload) as FoundryOutput;
    }

    /** @deprecated use Client::buildNftOutput() instead. */
    async buildNftOutput(data: BuildNftOutputData): Promise<NftOutput> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'buildNftOutput',
                data,
            },
        );
        return Output.parse(JSON.parse(response).payload) as NftOutput;
    }

    /**
     * A generic `burn()` function that can be used to prepare to burn native tokens, nfts, foundries and aliases.
     * @param burn The outputs to burn
     * @param transactionOptions The options to define a `RemainderValueStrategy`
     * or custom inputs.
     * @returns The prepared transaction.
     */
    async prepareBurn(
        burn: Burn,
        transactionOptions?: TransactionOptions,
    ): Promise<PreparedTransaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareBurn',
                data: {
                    burn,
                    options: transactionOptions,
                },
            },
        );
        const parsed = JSON.parse(
            response,
        ) as Response<PreparedTransactionData>;
        return new PreparedTransaction(
            plainToInstance(PreparedTransactionData, parsed.payload),
            this,
        );
    }

    /**
     * Burn native tokens. This doesn't require the foundry output which minted them, but will not increase
     * the foundries `melted_tokens` field, which makes it impossible to destroy the foundry output. Therefore it's
     * recommended to use melting, if the foundry output is available.
     * @param tokenId The native token id.
     * @param burnAmount The to be burned amount.
     * @param transactionOptions The options to define a `RemainderValueStrategy`
     * or custom inputs.
     * @returns The prepared transaction.
     */
    async prepareBurnNativeToken(
        tokenId: string,
        burnAmount: bigint,
        transactionOptions?: TransactionOptions,
    ): Promise<PreparedTransaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareBurn',
                data: {
                    burn: {
                        nativeTokens: new Map([[tokenId, burnAmount]]),
                    },
                    options: transactionOptions,
                },
            },
        );
        const parsed = JSON.parse(
            response,
        ) as Response<PreparedTransactionData>;
        return new PreparedTransaction(
            plainToInstance(PreparedTransactionData, parsed.payload),
            this,
        );
    }
    /**
     * Burn an nft output.
     * @param nftId The NftId.
     * @param transactionOptions The options to define a `RemainderValueStrategy`
     * or custom inputs.
     * @returns The prepared transaction.
     */
    async prepareBurnNft(
        nftId: string,
        transactionOptions?: TransactionOptions,
    ): Promise<PreparedTransaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareBurn',
                data: {
                    burn: {
                        nfts: [nftId],
                    },
                    options: transactionOptions,
                },
            },
        );
        const parsed = JSON.parse(
            response,
        ) as Response<PreparedTransactionData>;
        return new PreparedTransaction(
            plainToInstance(PreparedTransactionData, parsed.payload),
            this,
        );
    }

    /**
     * Claim basic or nft outputs that have additional unlock conditions
     * to their `AddressUnlockCondition` from the account.
     * @param outputIds The outputs to claim.
     * @returns The resulting transaction.
     */
    async claimOutputs(outputIds: string[]): Promise<Transaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'claimOutputs',
                data: {
                    outputIdsToClaim: outputIds,
                },
            },
        );
        const parsed = JSON.parse(response) as Response<Transaction>;
        return plainToInstance(Transaction, parsed.payload);
    }

    /**
     * Consolidate basic outputs with only an `AddressUnlockCondition` from an account
     * by sending them to an own address again if the output amount is greater or
     * equal to the output consolidation threshold.
     * @param force Force consolidation on addresses where the threshold isn't met.
     * @param outputConsolidationThreshold A default threshold is used if this is omitted.
     * @returns The prepared consolidation transaction.
     */
    async prepareConsolidateOutputs(
        params: ConsolidationParams,
    ): Promise<PreparedTransaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareConsolidateOutputs',
                data: {
                    params,
                },
            },
        );
        const parsed = JSON.parse(
            response,
        ) as Response<PreparedTransactionData>;
        return new PreparedTransaction(
            plainToInstance(PreparedTransactionData, parsed.payload),
            this,
        );
    }

    /**
     * `createAliasOutput` creates an alias output
     * @param params The alias output options.
     * @param transactionOptions The options to define a `RemainderValueStrategy`
     * or custom inputs.
     * @returns The prepared transaction.
     */
    async prepareCreateAliasOutput(
        params?: AliasOutputParams,
        transactionOptions?: TransactionOptions,
    ): Promise<PreparedTransaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareCreateAliasOutput',
                data: {
                    params,
                    options: transactionOptions,
                },
            },
        );
        const parsed = JSON.parse(
            response,
        ) as Response<PreparedTransactionData>;
        return new PreparedTransaction(
            plainToInstance(PreparedTransactionData, parsed.payload),
            this,
        );
    }

    /**
     * Melt native tokens. This happens with the foundry output which minted them, by increasing its
     * `melted_tokens` field.
     * @param tokenId The native token id.
     * @param meltAmount To be melted amount.
     * @param transactionOptions The options to define a `RemainderValueStrategy`
     * or custom inputs.
     * @returns The prepared transaction.
     */
    async prepareMeltNativeToken(
        tokenId: string,
        meltAmount: bigint,
        transactionOptions?: TransactionOptions,
    ): Promise<PreparedTransaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareMeltNativeToken',
                data: {
                    tokenId,
                    meltAmount: bigIntToHex(meltAmount),
                    options: transactionOptions,
                },
            },
        );
        const parsed = JSON.parse(
            response,
        ) as Response<PreparedTransactionData>;
        return new PreparedTransaction(
            plainToInstance(PreparedTransactionData, parsed.payload),
            this,
        );
    }

    /**
     * Deregister a participation event.
     *
     * @param eventId The id of the participation event to deregister.
     */
    async deregisterParticipationEvent(
        eventId: ParticipationEventId,
    ): Promise<void> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'deregisterParticipationEvent',
                data: {
                    eventId,
                },
            },
        );
        return JSON.parse(response).payload;
    }

    /**
     * Destroy an alias output.
     *
     * @param aliasId The AliasId.
     * @param transactionOptions The options to define a `RemainderValueStrategy`
     * or custom inputs.
     * @returns The prepared transaction.
     */
    async prepareDestroyAlias(
        aliasId: string,
        transactionOptions?: TransactionOptions,
    ): Promise<PreparedTransaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareBurn',
                data: {
                    burn: {
                        aliases: [aliasId],
                    },
                    options: transactionOptions,
                },
            },
        );
        const parsed = JSON.parse(
            response,
        ) as Response<PreparedTransactionData>;
        return new PreparedTransaction(
            plainToInstance(PreparedTransactionData, parsed.payload),
            this,
        );
    }

    /**
     * Function to destroy a foundry output with a circulating supply of 0.
     * Native tokens in the foundry (minted by other foundries) will be transacted to the controlling alias.
     *
     * @param foundryId The FoundryId.
     * @param transactionOptions The options to define a `RemainderValueStrategy`
     * or custom inputs.
     * @returns The prepared transaction.
     */
    async prepareDestroyFoundry(
        foundryId: string,
        transactionOptions?: TransactionOptions,
    ): Promise<PreparedTransaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareBurn',
                data: {
                    burn: {
                        foundries: [foundryId],
                    },
                    options: transactionOptions,
                },
            },
        );
        const parsed = JSON.parse(
            response,
        ) as Response<PreparedTransactionData>;
        return new PreparedTransaction(
            plainToInstance(PreparedTransactionData, parsed.payload),
            this,
        );
    }

    /**
     * Generate new unused Ed25519 addresses.
     *
     * @param amount The amount of addresses to generate.
     * @param options Options for address generation.
     * @returns The addresses.
     */
    async generateEd25519Addresses(
        amount: number,
        options?: GenerateAddressOptions,
    ): Promise<AccountAddress[]> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'generateEd25519Addresses',
                data: {
                    amount,
                    options,
                },
            },
        );
        return JSON.parse(response).payload;
    }

    /**
     * Get the account balance.
     *
     * @returns The account balance.
     */
    async getBalance(): Promise<Balance> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'getBalance',
            },
        );
        const payload = JSON.parse(response).payload;
        return this.adjustBalancePayload(payload);
    }

    /**
     * Converts hex encoded or decimal strings of amounts to `bigint`
     * for the balance payload.
     */
    private adjustBalancePayload(payload: any): Balance {
        for (let i = 0; i < payload.nativeTokens.length; i++) {
            payload.nativeTokens[i].total = hexToBigInt(
                payload.nativeTokens[i].total,
            );
            payload.nativeTokens[i].available = hexToBigInt(
                payload.nativeTokens[i].available,
            );
        }
        payload.baseCoin.total = BigInt(payload.baseCoin.total);
        payload.baseCoin.available = BigInt(payload.baseCoin.available);

        payload.requiredStorageDeposit.alias = BigInt(
            payload.requiredStorageDeposit.alias,
        );
        payload.requiredStorageDeposit.basic = BigInt(
            payload.requiredStorageDeposit.basic,
        );
        payload.requiredStorageDeposit.foundry = BigInt(
            payload.requiredStorageDeposit.foundry,
        );
        payload.requiredStorageDeposit.nft = BigInt(
            payload.requiredStorageDeposit.nft,
        );

        return payload;
    }

    /**
     * Get the data for an output.
     * @param outputId The output to get.
     * @returns The `OutputData`.
     */
    async getOutput(outputId: string): Promise<OutputData> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'getOutput',
                data: {
                    outputId,
                },
            },
        );
        const parsed = JSON.parse(response) as Response<OutputData>;
        return plainToInstance(OutputData, parsed.payload);
    }

    /**
     * Get a participation event.
     *
     * @param eventId The ID of the event to get.
     */
    async getParticipationEvent(
        eventId: ParticipationEventId,
    ): Promise<ParticipationEventWithNodes> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'getParticipationEvent',
                data: {
                    eventId,
                },
            },
        );
        return JSON.parse(response).payload;
    }

    /**
     * Get IDs of participation events of a certain type.
     *
     * @param node The node to get events from.
     * @param eventType The type of events to get.
     */
    async getParticipationEventIds(
        node: INode,
        eventType?: ParticipationEventType,
    ): Promise<ParticipationEventId[]> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'getParticipationEventIds',
                data: {
                    node,
                    eventType,
                },
            },
        );
        return JSON.parse(response).payload;
    }

    /**
     * Get all participation events.
     */
    async getParticipationEvents(): Promise<ParticipationEventMap> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'getParticipationEvents',
            },
        );
        return JSON.parse(response).payload;
    }

    /**
     * Get the participation event status by its ID.
     *
     * @param eventId The ID of the event status to get.
     */
    async getParticipationEventStatus(
        eventId: ParticipationEventId,
    ): Promise<ParticipationEventStatus> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'getParticipationEventStatus',
                data: {
                    eventId,
                },
            },
        );
        return JSON.parse(response).payload;
    }

    /**
     * Get a `FoundryOutput` by native token ID. It will try to get the foundry from
     * the account, if it isn't in the account it will try to get it from the node.
     *
     * @param tokenId The native token ID to get the foundry for.
     * @returns The `FoundryOutput` that minted the token.
     */
    async getFoundryOutput(tokenId: string): Promise<FoundryOutput> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'getFoundryOutput',
                data: {
                    tokenId,
                },
            },
        );
        return Output.parse(JSON.parse(response).payload) as FoundryOutput;
    }

    /**
     * Get outputs with additional unlock conditions.
     *
     * @param outputs The type of outputs to claim.
     * @returns The output IDs of the unlockable outputs.
     */
    async claimableOutputs(outputs: OutputsToClaim): Promise<string[]> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'claimableOutputs',
                data: {
                    outputsToClaim: outputs,
                },
            },
        );
        return JSON.parse(response).payload;
    }

    /**
     * Get a transaction stored in the account.
     *
     * @param transactionId The ID of the transaction to get.
     * @returns The transaction.
     */
    async getTransaction(transactionId: string): Promise<Transaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'getTransaction',
                data: {
                    transactionId,
                },
            },
        );
        const parsed = JSON.parse(response) as Response<Transaction>;
        return plainToInstance(Transaction, parsed.payload);
    }

    /**
     * Get the transaction with inputs of an incoming transaction stored in the account
     * List might not be complete, if the node pruned the data already
     *
     * @param transactionId The ID of the transaction to get.
     * @returns The transaction.
     */
    async getIncomingTransaction(transactionId: string): Promise<Transaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'getIncomingTransaction',
                data: {
                    transactionId,
                },
            },
        );
        const parsed = JSON.parse(response) as Response<Transaction>;
        return plainToInstance(Transaction, parsed.payload);
    }

    /**
     * List all the addresses of the account.
     *
     * @returns The addresses.
     */
    async addresses(): Promise<AccountAddress[]> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'addresses',
            },
        );

        return JSON.parse(response).payload;
    }

    /**
     * List the addresses of the account with unspent outputs.
     *
     * @returns The addresses.
     */
    async addressesWithUnspentOutputs(): Promise<AddressWithUnspentOutputs[]> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'addressesWithUnspentOutputs',
            },
        );

        return JSON.parse(response).payload;
    }

    /**
     * List all outputs of the account.
     *
     * @param filterOptions Options to filter the to be returned outputs.
     * @returns The outputs with metadata.
     */
    async outputs(filterOptions?: FilterOptions): Promise<OutputData[]> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'outputs',
                data: { filterOptions },
            },
        );

        const parsed = JSON.parse(response) as Response<OutputData[]>;
        return plainToInstance(OutputData, parsed.payload);
    }

    /**
     * List all the pending transactions of the account.
     *
     * @returns The transactions.
     */
    async pendingTransactions(): Promise<Transaction[]> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'pendingTransactions',
            },
        );
        const parsed = JSON.parse(response) as Response<Transaction[]>;
        return plainToInstance(Transaction, parsed.payload);
    }

    /**
     * List all incoming transactions of the account.
     *
     * @returns The incoming transactions with their inputs.
     */
    async incomingTransactions(): Promise<Transaction[]> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'incomingTransactions',
            },
        );
        const parsed = JSON.parse(response) as Response<Transaction[]>;
        return plainToInstance(Transaction, parsed.payload);
    }

    /**
     * List all the transactions of the account.
     *
     * @returns The transactions.
     */
    async transactions(): Promise<Transaction[]> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'transactions',
            },
        );
        const parsed = JSON.parse(response) as Response<Transaction[]>;
        return plainToInstance(Transaction, parsed.payload);
    }

    /**
     * List all the unspent outputs of the account.
     *
     * @param filterOptions Options to filter the to be returned outputs.
     * @returns The outputs with metadata.
     */
    async unspentOutputs(filterOptions?: FilterOptions): Promise<OutputData[]> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'unspentOutputs',
                data: { filterOptions },
            },
        );
        const parsed = JSON.parse(response) as Response<OutputData[]>;
        return plainToInstance(OutputData, parsed.payload);
    }

    /**
     * Get the accounts metadata.
     *
     * @returns The accounts metadata.
     */
    getMetadata(): AccountMetadata {
        return {
            alias: this.meta.alias,
            coinType: this.meta.coinType,
            index: this.meta.index,
        };
    }

    /**
     * Mint additional native tokens.
     *
     * @param tokenId The native token id.
     * @param mintAmount To be minted amount.
     * @param transactionOptions The options to define a `RemainderValueStrategy`
     * or custom inputs.
     * @returns The prepared minting transaction.
     */
    async prepareMintNativeToken(
        tokenId: string,
        mintAmount: bigint,
        transactionOptions?: TransactionOptions,
    ): Promise<PreparedTransaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareMintNativeToken',
                data: {
                    tokenId,
                    mintAmount: bigIntToHex(mintAmount),
                    options: transactionOptions,
                },
            },
        );

        const parsed = JSON.parse(
            response,
        ) as Response<PreparedTransactionData>;
        return new PreparedTransaction(
            plainToInstance(PreparedTransactionData, parsed.payload),
            this,
        );
    }

    /**
     * Create a native token.
     *
     * @param params The options for creating a native token.
     * @param transactionOptions The options to define a `RemainderValueStrategy`
     * or custom inputs.
     * @returns The creating transaction and the token ID.
     */
    async prepareCreateNativeToken(
        params: CreateNativeTokenParams,
        transactionOptions?: TransactionOptions,
    ): Promise<PreparedCreateNativeTokenTransaction> {
        const adjustedParams: any = params;
        adjustedParams.circulatingSupply = bigIntToHex(
            params.circulatingSupply,
        );
        adjustedParams.maximumSupply = bigIntToHex(params.maximumSupply);

        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareCreateNativeToken',
                data: {
                    params: adjustedParams,
                    options: transactionOptions,
                },
            },
        );

        const parsed = JSON.parse(
            response,
        ) as Response<PreparedCreateNativeTokenTransactionData>;
        return new PreparedCreateNativeTokenTransaction(
            plainToInstance(
                PreparedCreateNativeTokenTransactionData,
                parsed.payload,
            ),
            this,
        );
    }

    /**
     * Mint NFTs.
     *
     * @param params The options for minting nfts.
     * @param transactionOptions The options to define a `RemainderValueStrategy`
     * or custom inputs.
     * @returns The prepared minting transaction.
     */
    async prepareMintNfts(
        params: MintNftParams[],
        transactionOptions?: TransactionOptions,
    ): Promise<PreparedTransaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareMintNfts',
                data: {
                    params,
                    options: transactionOptions,
                },
            },
        );

        const parsed = JSON.parse(
            response,
        ) as Response<PreparedTransactionData>;
        return new PreparedTransaction(
            plainToInstance(PreparedTransactionData, parsed.payload),
            this,
        );
    }

    /**
     * Prepare an output for sending, useful for offline signing.
     *
     * @param options The options for preparing an output. If the amount is
     * below the minimum required storage deposit, by default the remaining
     * amount will automatically be added with a `StorageDepositReturn` `UnlockCondition`,
     * when setting the `ReturnStrategy` to `gift`, the full minimum required
     * storage deposit will be sent to the recipient. When the assets contain
     * an nft id, the data from the existing `NftOutput` will be used, just with
     * the address unlock conditions replaced.
     * @param transactionOptions The options to define a `RemainderValueStrategy`
     * or custom inputs.
     * @returns The prepared output.
     */
    async prepareOutput(
        params: OutputParams,
        transactionOptions?: TransactionOptions,
    ): Promise<Output> {
        if (typeof params.amount === 'bigint') {
            params.amount = params.amount.toString(10);
        }

        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareOutput',
                data: {
                    params,
                    transactionOptions,
                },
            },
        );

        return Output.parse(JSON.parse(response).payload);
    }

    /**
     * Prepare to send base coins, useful for offline signing.
     *
     * @param params Address with amounts to send.
     * @param options The options to define a `RemainderValueStrategy`
     * or custom inputs.
     * @returns The prepared transaction data.
     */
    async prepareSend(
        params: SendParams[],
        options?: TransactionOptions,
    ): Promise<PreparedTransaction> {
        for (let i = 0; i < params.length; i++) {
            if (typeof params[i].amount === 'bigint') {
                params[i].amount = params[i].amount.toString(10);
            }
        }
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareSend',
                data: {
                    params,
                    options,
                },
            },
        );
        const parsed = JSON.parse(
            response,
        ) as Response<PreparedTransactionData>;
        return new PreparedTransaction(
            plainToInstance(PreparedTransactionData, parsed.payload),
            this,
        );
    }

    /**
     * Prepare a transaction, useful for offline signing.
     *
     * @param outputs Outputs to use in the transaction.
     * @param options The options to define a `RemainderValueStrategy`
     * or custom inputs.
     * @returns The prepared transaction data.
     */
    async prepareTransaction(
        outputs: Output[],
        options?: TransactionOptions,
    ): Promise<PreparedTransaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareTransaction',
                data: {
                    outputs,
                    options,
                },
            },
        );
        const parsed = JSON.parse(
            response,
        ) as Response<PreparedTransactionData>;
        return new PreparedTransaction(
            plainToInstance(PreparedTransactionData, parsed.payload),
            this,
        );
    }

    /**
     * Register participation events.
     *
     * @param options Options to register participation events.
     * @returns A mapping between event IDs and their corresponding event data.
     */
    async registerParticipationEvents(
        options: ParticipationEventRegistrationOptions,
    ): Promise<ParticipationEventMap> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'registerParticipationEvents',
                data: {
                    options,
                },
            },
        );
        return JSON.parse(response).payload;
    }

    /**
     * Retries (promotes or reattaches) a transaction sent from the account for a provided transaction id until it's
     * included (referenced by a milestone). Returns the included block id.
     */
    async retryTransactionUntilIncluded(
        transactionId: string,
        interval?: number,
        maxAttempts?: number,
    ): Promise<string> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'retryTransactionUntilIncluded',
                data: {
                    transactionId,
                    interval,
                    maxAttempts,
                },
            },
        );
        return JSON.parse(response).payload;
    }

    /**
     * Send base coins to an address.
     *
     * @param amount Amount of coins.
     * @param address Receiving address.
     * @param transactionOptions The options to define a `RemainderValueStrategy`
     * or custom inputs.
     * @returns The sent transaction.
     */
    async send(
        amount: bigint | string,
        address: string,
        transactionOptions?: TransactionOptions,
    ): Promise<Transaction> {
        if (typeof amount === 'bigint') {
            amount = amount.toString(10);
        }
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'send',
                data: {
                    amount,
                    address,
                    options: transactionOptions,
                },
            },
        );
        const parsed = JSON.parse(response) as Response<Transaction>;
        return plainToInstance(Transaction, parsed.payload);
    }

    /**
     * Send base coins with amounts from input addresses.
     *
     * @param params Addresses with amounts.
     * @param transactionOptions The options to define a `RemainderValueStrategy`
     * or custom inputs.
     * @returns The sent transaction.
     */
    async sendWithParams(
        params: SendParams[],
        transactionOptions?: TransactionOptions,
    ): Promise<Transaction> {
        for (let i = 0; i < params.length; i++) {
            if (typeof params[i].amount === 'bigint') {
                params[i].amount = params[i].amount.toString(10);
            }
        }
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'sendWithParams',
                data: {
                    params,
                    options: transactionOptions,
                },
            },
        );
        const parsed = JSON.parse(response) as Response<Transaction>;
        return plainToInstance(Transaction, parsed.payload);
    }

    /**
     * Send native tokens.
     *
     * @param params Addresses amounts and native tokens.
     * @param transactionOptions The options to define a `RemainderValueStrategy`
     * or custom inputs.
     * @returns The prepared transaction.
     */
    async prepareSendNativeTokens(
        params: SendNativeTokensParams[],
        transactionOptions?: TransactionOptions,
    ): Promise<PreparedTransaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareSendNativeTokens',
                data: {
                    params,
                    options: transactionOptions,
                },
            },
        );
        const parsed = JSON.parse(
            response,
        ) as Response<PreparedTransactionData>;
        return new PreparedTransaction(
            plainToInstance(PreparedTransactionData, parsed.payload),
            this,
        );
    }

    /**
     * Send NFT.
     *
     * @param params Addresses and nft ids.
     * @param transactionOptions The options to define a `RemainderValueStrategy`
     * or custom inputs.
     * @returns The prepared transaction.
     */
    async prepareSendNft(
        params: SendNftParams[],
        transactionOptions?: TransactionOptions,
    ): Promise<PreparedTransaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareSendNft',
                data: {
                    params,
                    options: transactionOptions,
                },
            },
        );
        const parsed = JSON.parse(
            response,
        ) as Response<PreparedTransactionData>;
        return new PreparedTransaction(
            plainToInstance(PreparedTransactionData, parsed.payload),
            this,
        );
    }

    /**
     * Send outputs in a transaction.
     *
     * @param outputs The outputs to send.
     * @param transactionOptions The options to define a `RemainderValueStrategy`
     * or custom inputs.
     * @returns The sent transaction.
     */
    async sendOutputs(
        outputs: Output[],
        transactionOptions?: TransactionOptions,
    ): Promise<Transaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'sendOutputs',
                data: {
                    outputs,
                    options: transactionOptions,
                },
            },
        );

        const parsed = JSON.parse(response) as Response<Transaction>;
        return plainToInstance(Transaction, parsed.payload);
    }

    /**
     * Set the alias for the account
     *
     * @param alias The account alias to set.
     */
    async setAlias(alias: string): Promise<void> {
        await this.methodHandler.callAccountMethod(this.meta.index, {
            name: 'setAlias',
            data: {
                alias,
            },
        });
    }

    /**
     * Set the fallback SyncOptions for account syncing.
     * If storage is enabled, will persist during restarts.
     *
     * @param options The sync options to set.
     */
    async setDefaultSyncOptions(options: SyncOptions): Promise<void> {
        await this.methodHandler.callAccountMethod(this.meta.index, {
            name: 'setDefaultSyncOptions',
            data: {
                options,
            },
        });
    }

    /**
     * Sign a prepared transaction, useful for offline signing.
     *
     * @param preparedTransactionData The prepared transaction data to sign.
     * @returns The signed transaction essence.
     */
    async signTransactionEssence(
        preparedTransactionData: PreparedTransactionData,
    ): Promise<SignedTransactionEssence> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'signTransactionEssence',
                data: {
                    preparedTransactionData,
                },
            },
        );
        const parsed = JSON.parse(
            response,
        ) as Response<SignedTransactionEssence>;
        return plainToInstance(SignedTransactionEssence, parsed.payload);
    }

    /**
     * Sign a prepared transaction, and send it.
     *
     * @param preparedTransactionData The prepared transaction data to sign and submit.
     * @returns The transaction.
     */
    async signAndSubmitTransaction(
        preparedTransactionData: PreparedTransactionData,
    ): Promise<Transaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'signAndSubmitTransaction',
                data: {
                    preparedTransactionData,
                },
            },
        );
        const parsed = JSON.parse(response) as Response<Transaction>;
        return plainToInstance(Transaction, parsed.payload);
    }

    /**
     * Validate the transaction, submit it to a node and store it in the account.
     *
     * @param signedTransactionData A signed transaction to submit and store.
     * @returns The sent transaction.
     */
    async submitAndStoreTransaction(
        signedTransactionData: SignedTransactionEssence,
    ): Promise<Transaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'submitAndStoreTransaction',
                data: {
                    signedTransactionData,
                },
            },
        );
        const parsed = JSON.parse(response) as Response<Transaction>;
        return plainToInstance(Transaction, parsed.payload);
    }

    /**
     * Sync the account by fetching new information from the nodes.
     * Will also retry pending transactions if necessary.
     * A custom default can be set using setDefaultSyncOptions.
     *
     * @param options Optional synchronization options.
     * @returns The account balance.
     */
    async sync(options?: SyncOptions): Promise<Balance> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'sync',
                data: {
                    options,
                },
            },
        );
        const payload = JSON.parse(response).payload;
        return this.adjustBalancePayload(payload);
    }

    /**
     * Prepare a vote.
     *
     * @param eventId The participation event ID.
     * @param answers Answers for a voting event, can be empty.
     * @returns An instance of `PreparedTransaction`.
     */
    async prepareVote(
        eventId?: ParticipationEventId,
        answers?: number[],
    ): Promise<PreparedTransaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareVote',
                data: {
                    eventId,
                    answers,
                },
            },
        );
        const parsed = JSON.parse(
            response,
        ) as Response<PreparedTransactionData>;
        return new PreparedTransaction(
            plainToInstance(PreparedTransactionData, parsed.payload),
            this,
        );
    }

    /**
     * Prepare stop participating in an event.
     *
     * @param eventId The event ID to stop participating in.
     * @returns An instance of `PreparedTransaction`.
     */
    async prepareStopParticipating(
        eventId: ParticipationEventId,
    ): Promise<PreparedTransaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareStopParticipating',
                data: {
                    eventId,
                },
            },
        );
        const parsed = JSON.parse(
            response,
        ) as Response<PreparedTransactionData>;
        return new PreparedTransaction(
            plainToInstance(PreparedTransactionData, parsed.payload),
            this,
        );
    }

    /**
     * Calculates the voting overview of an account.
     *
     * @param eventIds Optional, filters participations only for provided events.
     * @returns An instance of `ParticipationOverview`
     */
    async getParticipationOverview(
        eventIds?: ParticipationEventId[],
    ): Promise<ParticipationOverview> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'getParticipationOverview',
                data: {
                    eventIds,
                },
            },
        );
        return JSON.parse(response).payload;
    }

    /** @deprecated use prepareIncreaseVotingPower() instead. */
    async prepareVotingPower(amount: string): Promise<PreparedTransaction> {
        return this.prepareIncreaseVotingPower(amount);
    }

    /**
     * Prepare to increase the voting power.
     *
     * @param amount The amount to increase the voting power by.
     * @returns An instance of `PreparedTransaction`.
     */
    async prepareIncreaseVotingPower(
        amount: string,
    ): Promise<PreparedTransaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareIncreaseVotingPower',
                data: {
                    amount,
                },
            },
        );
        const parsed = JSON.parse(
            response,
        ) as Response<PreparedTransactionData>;
        return new PreparedTransaction(
            plainToInstance(PreparedTransactionData, parsed.payload),
            this,
        );
    }

    /**
     * Prepare to decrease the voting power.
     *
     * @param amount The amount to decrease the voting power by.
     * @returns An instance of `PreparedTransaction`.
     */
    async prepareDecreaseVotingPower(
        amount: string,
    ): Promise<PreparedTransaction> {
        const response = await this.methodHandler.callAccountMethod(
            this.meta.index,
            {
                name: 'prepareDecreaseVotingPower',
                data: {
                    amount,
                },
            },
        );
        const parsed = JSON.parse(
            response,
        ) as Response<PreparedTransactionData>;
        return new PreparedTransaction(
            plainToInstance(PreparedTransactionData, parsed.payload),
            this,
        );
    }
}
