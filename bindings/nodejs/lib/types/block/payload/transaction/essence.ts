// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { Type } from 'class-transformer';
import { PayloadDiscriminator } from '..';
import { HexEncodedString } from '../../../utils';
import { Input, InputDiscriminator } from '../../input';
import { Output, OutputDiscriminator } from '../../output';
import { Payload } from '../payload';

/**
 * All of the essence types.
 */
enum TransactionEssenceType {
    /**
     * A regular transaction essence.
     */
    Regular = 1,
}

/**
 * The base class for transaction essences.
 */
abstract class TransactionEssence {
    readonly type: TransactionEssenceType;

    /**
     * @param type The type of transaction essence.
     */
    constructor(type: TransactionEssenceType) {
        this.type = type;
    }

    /**
     * Get the type of essence.
     */
    getType(): TransactionEssenceType {
        return this.type;
    }
}

/**
 * RegularTransactionEssence transaction essence.
 */
class RegularTransactionEssence extends TransactionEssence {
    /// The unique value denoting whether the block was meant for mainnet, testnet, or a private network.
    networkId: number;
    inputsCommitment: HexEncodedString;

    @Type(() => Input, {
        discriminator: InputDiscriminator,
    })
    inputs: Input[];

    @Type(() => Output, {
        discriminator: OutputDiscriminator,
    })
    outputs: Output[];

    @Type(() => Payload, {
        discriminator: PayloadDiscriminator,
    })
    payload: Payload | undefined;

    /**
     * @param networkId The ID of the network the transaction was issued to.
     * @param inputsCommitment The hash of all inputs.
     * @param inputs The inputs of the transaction.
     * @param outputs The outputs of the transaction.
     * @param payload An optional Tagged Data payload.
     *
     */
    constructor(
        networkId: number,
        inputsCommitment: HexEncodedString,
        inputs: Input[],
        outputs: Output[],
        payload: Payload | undefined,
    ) {
        super(TransactionEssenceType.Regular);
        this.networkId = networkId;
        this.inputsCommitment = inputsCommitment;
        this.inputs = inputs;
        this.outputs = outputs;
        this.payload = payload;
    }
}

const TransactionEssenceDiscriminator = {
    property: 'type',
    subTypes: [
        {
            value: RegularTransactionEssence,
            name: TransactionEssenceType.Regular as any,
        },
    ],
};

export {
    TransactionEssenceDiscriminator,
    TransactionEssence,
    TransactionEssenceType,
    RegularTransactionEssence,
};
