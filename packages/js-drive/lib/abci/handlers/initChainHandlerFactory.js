const {
  tendermint: {
    abci: {
      ResponseInitChain,
    },
  },
} = require('@dashevo/abci/types');

const BlockInfo = require('../../blockExecution/BlockInfo');
const protoTimestampToMillis = require('../../util/protoTimestampToMillis');

/**
 * Init Chain ABCI handler
 *
 * @param {updateSimplifiedMasternodeList} updateSimplifiedMasternodeList
 * @param {number} initialCoreChainLockedHeight
 * @param {ValidatorSet} validatorSet
 * @param {createValidatorSetUpdate} createValidatorSetUpdate
 * @param {synchronizeMasternodeIdentities} synchronizeMasternodeIdentities
 * @param {BaseLogger} logger
 * @param {registerSystemDataContracts} registerSystemDataContracts
 * @param {GroveDBStore} groveDBStore
 * @param {RSAbci} rsAbci
 * @param {createCoreChainLockUpdate} createCoreChainLockUpdate
 * @return {initChainHandler}
 */
function initChainHandlerFactory(
  updateSimplifiedMasternodeList,
  initialCoreChainLockedHeight,
  validatorSet,
  createValidatorSetUpdate,
  synchronizeMasternodeIdentities,
  logger,
  registerSystemDataContracts,
  groveDBStore,
  rsAbci,
  createCoreChainLockUpdate,
) {
  /**
   * @typedef initChainHandler
   *
   * @param {abci.RequestInitChain} request
   * @return {Promise<abci.ResponseInitChain>}
   */
  async function initChainHandler(request) {
    const { time } = request;

    const consensusLogger = logger.child({
      height: request.initialHeight.toString(),
      abciMethod: 'initChain',
    });

    consensusLogger.debug('InitChain ABCI method requested');
    consensusLogger.trace({ abciRequest: request });

    await updateSimplifiedMasternodeList(
      initialCoreChainLockedHeight, {
        logger: consensusLogger,
      },
    );

    // Create initial state

    await groveDBStore.startTransaction();

    // Call RS ABCI

    logger.debug('Request RS Drive\'s InitChain method');

    await rsAbci.initChain({ }, true);

    const blockInfo = new BlockInfo(
      0,
      0,
      protoTimestampToMillis(time),
    );

    await registerSystemDataContracts(consensusLogger, blockInfo);

    const synchronizeMasternodeIdentitiesResult = await synchronizeMasternodeIdentities(
      initialCoreChainLockedHeight,
      blockInfo,
    );

    const {
      createdEntities, updatedEntities, removedEntities, fromHeight, toHeight,
    } = synchronizeMasternodeIdentitiesResult;

    consensusLogger.info(
      `Masternode identities are synced for heights from ${fromHeight} to ${toHeight}: ${createdEntities.length} created, ${updatedEntities.length} updated, ${removedEntities.length} removed`,
    );

    consensusLogger.trace(
      {
        createdEntities: createdEntities.map((item) => item.toJSON()),
        updatedEntities: updatedEntities.map((item) => item.toJSON()),
        removedEntities: removedEntities.map((item) => item.toJSON()),
      },
      'Synchronized masternode identities',
    );

    // Set initial validator set

    await validatorSet.initialize(initialCoreChainLockedHeight);

    const { quorumHash } = validatorSet.getQuorum();

    const validatorSetUpdate = createValidatorSetUpdate(validatorSet);

    const coreChainLockUpdate = await createCoreChainLockUpdate(
      initialCoreChainLockedHeight,
      0,
      consensusLogger,
    );

    const appHash = await groveDBStore.getRootHash({ useTransaction: true });

    await groveDBStore.commitTransaction();

    consensusLogger.trace(validatorSetUpdate, `Validator set initialized with ${quorumHash} quorum`);

    consensusLogger.info(
      {
        chainId: request.chainId,
        appHash: appHash.toString('hex').toUpperCase(),
        initialHeight: request.initialHeight.toString(),
        initialCoreHeight: initialCoreChainLockedHeight,
      },
      `Init ${request.chainId} chain on block #${request.initialHeight.toString()} with app hash ${appHash.toString('hex').toUpperCase()}`,
    );

    return new ResponseInitChain({
      appHash,
      validatorSetUpdate,
      initialCoreHeight: initialCoreChainLockedHeight,
      nextCoreChainLockUpdate: coreChainLockUpdate,
    });
  }

  return initChainHandler;
}

module.exports = initChainHandlerFactory;
