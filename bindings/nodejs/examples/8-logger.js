/**
 * This example initializes the logger.
 */

const getUnlockedManager = require('./account-manager');

async function run() {
    const { initLogger } = require('@iota/wallet');
    initLogger({
        color_enabled: true,
        outputs: [
            {
                name: './wallet.log',
                level_filter: 'debug',
            },
        ],
    });

    try {
        const manager = await getUnlockedManager();
        const account = await manager.getAccount('Alice');
        console.log('Account:', account);
    } catch (error) {
        console.log('Error: ' + error);
    }
    process.exit(0);
}

run();
