const {
  createContainer: createAwilixContainer,
  InjectionMode,
  asFunction,
  asValue,
  asClass,
} = require('awilix');

const Docker = require('dockerode');

const ensureHomeDirFactory = require('./ensureHomeDirFactory');
const ConfigFileJsonRepository = require('./config/configFile/ConfigFileJsonRepository');
const createSystemConfigsFactory = require('./config/systemConfigs/createSystemConfigsFactory');
const isSystemConfigFactory = require('./config/systemConfigs/isSystemConfigFactory');
const migrateConfigFile = require('./config/configFile/migrateConfigFile');
const systemConfigs = require('../configs/system');

const renderServiceTemplatesFactory = require('./templates/renderServiceTemplatesFactory');
const writeServiceConfigsFactory = require('./templates/writeServiceConfigsFactory');

const DockerCompose = require('./docker/DockerCompose');
const StartedContainers = require('./docker/StartedContainers');
const stopAllContainersFactory = require('./docker/stopAllContainersFactory');
const dockerPullFactory = require('./docker/dockerPullFactory');
const resolveDockerHostIpFactory = require('./docker/resolveDockerHostIpFactory');

const startCoreFactory = require('./core/startCoreFactory');
const createRpcClient = require('./core/createRpcClient');
const waitForCoreStart = require('./core/waitForCoreStart');
const waitForCoreSync = require('./core/waitForCoreSync');
const waitForMasternodesSync = require('./core/waitForMasternodesSync');
const waitForBlocks = require('./core/waitForBlocks');
const waitForConfirmations = require('./core/waitForConfirmations');
const generateBlsKeys = require('./core/generateBlsKeys');
const activateCoreSpork = require('./core/activateCoreSpork');
const waitForCorePeersConnected = require('./core/waitForCorePeersConnected');

const createNewAddress = require('./core/wallet/createNewAddress');
const generateBlocks = require('./core/wallet/generateBlocks');
const generateToAddress = require('./core/wallet/generateToAddress');
const importPrivateKey = require('./core/wallet/importPrivateKey');
const getAddressBalance = require('./core/wallet/getAddressBalance');
const sendToAddress = require('./core/wallet/sendToAddress');
const registerMasternode = require('./core/wallet/registerMasternode');
const waitForBalanceToConfirm = require('./core/wallet/waitForBalanceToConfirm');

const getCoreScopeFactory = require('./status/scopes/core');
const getMasternodeScopeFactory = require('./status/scopes/masternode');
const getPlatformScopeFactory = require('./status/scopes/platform');
const getOverviewScopeFactory = require('./status/scopes/overview');
const getServicesScopeFactory = require('./status/scopes/services');
const getHostScopeFactory = require('./status/scopes/host');

const generateToAddressTaskFactory = require('./listr/tasks/wallet/generateToAddressTaskFactory');
const registerMasternodeTaskFactory = require('./listr/tasks/registerMasternodeTaskFactory');
const featureFlagTaskFactory = require('./listr/tasks/platform/featureFlagTaskFactory');
const tenderdashInitTaskFactory = require('./listr/tasks/platform/tenderdashInitTaskFactory');
const startNodeTaskFactory = require('./listr/tasks/startNodeTaskFactory');

const createTenderdashRpcClient = require('./tenderdash/createTenderdashRpcClient');
const initializeTenderdashNodeFactory = require('./tenderdash/initializeTenderdashNodeFactory');
const setupLocalPresetTaskFactory = require('./listr/tasks/setup/setupLocalPresetTaskFactory');
const setupRegularPresetTaskFactory = require('./listr/tasks/setup/setupRegularPresetTaskFactory');
const stopNodeTaskFactory = require('./listr/tasks/stopNodeTaskFactory');
const restartNodeTaskFactory = require('./listr/tasks/restartNodeTaskFactory');
const resetNodeTaskFactory = require('./listr/tasks/resetNodeTaskFactory');
const configureCoreTaskFactory = require('./listr/tasks/setup/local/configureCoreTaskFactory');
const configureTenderdashTaskFactory = require('./listr/tasks/setup/local/configureTenderdashTaskFactory');
const obtainSelfSignedCertificateTaskFactory = require('./listr/tasks/ssl/selfSigned/obtainSelfSignedCertificateTaskFactory');
const waitForNodeToBeReadyTaskFactory = require('./listr/tasks/platform/waitForNodeToBeReadyTaskFactory');
const enableCoreQuorumsTaskFactory = require('./listr/tasks/setup/local/enableCoreQuorumsTaskFactory');
const startGroupNodesTaskFactory = require('./listr/tasks/startGroupNodesTaskFactory');
const buildServicesTaskFactory = require('./listr/tasks/buildServicesTaskFactory');
const reindexNodeTaskFactory = require('./listr/tasks/reindexNodeTaskFactory');

const generateHDPrivateKeys = require('./util/generateHDPrivateKeys');

const obtainZeroSSLCertificateTaskFactory = require('./listr/tasks/ssl/zerossl/obtainZeroSSLCertificateTaskFactory');
const renewZeroSSLCertificateTaskFactory = require('./listr/tasks/ssl/zerossl/renewZeroSSLCertificateTaskFactory');
const VerificationServer = require('./listr/tasks/ssl/VerificationServer');
const saveCertificateTask = require('./listr/tasks/ssl/saveCertificateTask');

const createZeroSSLCertificate = require('./ssl/zerossl/createCertificate');
const verifyDomain = require('./ssl/zerossl/verifyDomain');
const downloadCertificate = require('./ssl/zerossl/downloadCertificate');
const getCertificate = require('./ssl/zerossl/getCertificate');
const listCertificates = require('./ssl/zerossl/listCertificates');
const generateCsr = require('./ssl/zerossl/generateCsr');
const generateKeyPair = require('./ssl/generateKeyPair');
const createSelfSignedCertificate = require('./ssl/selfSigned/createCertificate');

const scheduleRenewZeroSslCertificateFactory = require('./helper/scheduleRenewZeroSslCertificateFactory');

async function createDIContainer() {
  const container = createAwilixContainer({
    injectionMode: InjectionMode.CLASSIC,
  });

  /**
   * Config
   */
  container.register({
    ensureHomeDir: asFunction(ensureHomeDirFactory).singleton(),
    configFileRepository: asClass(ConfigFileJsonRepository).singleton(),
    systemConfigs: asValue(systemConfigs),
    createSystemConfigs: asFunction(createSystemConfigsFactory).singleton(),
    isSystemConfig: asFunction(isSystemConfigFactory).singleton(),
    migrateConfigFile: asValue(migrateConfigFile),
    // `configFile` and `config` are registering on command init
  });

  /**
   * Utils
   */
  container.register({
    generateHDPrivateKeys: asValue(generateHDPrivateKeys),
  });

  /**
   * Templates
   */
  container.register({
    renderServiceTemplates: asFunction(renderServiceTemplatesFactory).singleton(),
    writeServiceConfigs: asFunction(writeServiceConfigsFactory).singleton(),
  });

  /**
   * SSL
   */
  container.register({
    createZeroSSLCertificate: asValue(createZeroSSLCertificate),
    generateCsr: asValue(generateCsr),
    generateKeyPair: asValue(generateKeyPair),
    verifyDomain: asValue(verifyDomain),
    downloadCertificate: asValue(downloadCertificate),
    getCertificate: asValue(getCertificate),
    listCertificates: asValue(listCertificates),
    createSelfSignedCertificate: asValue(createSelfSignedCertificate),
    verificationServer: asClass(VerificationServer).singleton(),
  });

  /**
   * Docker
   */
  container.register({
    docker: asFunction(() => (
      new Docker()
    )).singleton(),
    dockerCompose: asClass(DockerCompose).singleton(),
    startedContainers: asFunction(() => (
      new StartedContainers()
    )).singleton(),
    stopAllContainers: asFunction(stopAllContainersFactory).singleton(),
    dockerPull: asFunction(dockerPullFactory).singleton(),
    resolveDockerHostIp: asFunction(resolveDockerHostIpFactory).singleton(),
  });

  /**
   * Core
   */
  container.register({
    createRpcClient: asValue(createRpcClient),
    waitForCoreStart: asValue(waitForCoreStart),
    waitForCoreSync: asValue(waitForCoreSync),
    waitForMasternodesSync: asValue(waitForMasternodesSync),
    startCore: asFunction(startCoreFactory).singleton(),
    waitForBlocks: asValue(waitForBlocks),
    waitForConfirmations: asValue(waitForConfirmations),
    generateBlsKeys: asValue(generateBlsKeys),
    activateCoreSpork: asValue(activateCoreSpork),
    waitForCorePeersConnected: asValue(waitForCorePeersConnected),
  });

  /**
   * Core Wallet
   */
  container.register({
    createNewAddress: asValue(createNewAddress),
    generateBlocks: asValue(generateBlocks),
    generateToAddress: asValue(generateToAddress),
    importPrivateKey: asValue(importPrivateKey),
    getAddressBalance: asValue(getAddressBalance),
    sendToAddress: asValue(sendToAddress),
    registerMasternode: asValue(registerMasternode),
    waitForBalanceToConfirm: asValue(waitForBalanceToConfirm),
  });

  /**
   * Tenderdash
   */
  container.register({
    createTenderdashRpcClient: asValue(createTenderdashRpcClient),
    initializeTenderdashNode: asFunction(initializeTenderdashNodeFactory).singleton(),
  });

  /**
   * Tasks
   */
  container.register({
    buildServicesTask: asFunction(buildServicesTaskFactory).singleton(),
    startGroupNodesTask: asFunction(startGroupNodesTaskFactory).singleton(),
    generateToAddressTask: asFunction(generateToAddressTaskFactory).singleton(),
    registerMasternodeTask: asFunction(registerMasternodeTaskFactory).singleton(),
    featureFlagTask: asFunction(featureFlagTaskFactory).singleton(),
    tenderdashInitTask: asFunction(tenderdashInitTaskFactory).singleton(),
    startNodeTask: asFunction(startNodeTaskFactory).singleton(),
    stopNodeTask: asFunction(stopNodeTaskFactory).singleton(),
    restartNodeTask: asFunction(restartNodeTaskFactory).singleton(),
    resetNodeTask: asFunction(resetNodeTaskFactory).singleton(),
    setupLocalPresetTask: asFunction(setupLocalPresetTaskFactory).singleton(),
    setupRegularPresetTask: asFunction(setupRegularPresetTaskFactory).singleton(),
    configureCoreTask: asFunction(configureCoreTaskFactory).singleton(),
    configureTenderdashTask: asFunction(configureTenderdashTaskFactory).singleton(),
    waitForNodeToBeReadyTask: asFunction(waitForNodeToBeReadyTaskFactory).singleton(),
    enableCoreQuorumsTask: asFunction(enableCoreQuorumsTaskFactory).singleton(),
    obtainZeroSSLCertificateTask: asFunction(obtainZeroSSLCertificateTaskFactory).singleton(),
    renewZeroSSLCertificateTask: asFunction(renewZeroSSLCertificateTaskFactory).singleton(),
    obtainSelfSignedCertificateTask: asFunction(obtainSelfSignedCertificateTaskFactory).singleton(),
    saveCertificateTask: asValue(saveCertificateTask),
    reindexNodeTask: asFunction(reindexNodeTaskFactory).singleton(),
    getCoreScope: asFunction(getCoreScopeFactory).singleton(),
    getMasternodeScope: asFunction(getMasternodeScopeFactory).singleton(),
    getPlatformScope: asFunction(getPlatformScopeFactory).singleton(),
    getOverviewScope: asFunction(getOverviewScopeFactory).singleton(),
    getServicesScope: asFunction(getServicesScopeFactory).singleton(),
    getHostScope: asFunction(getHostScopeFactory).singleton(),
  });

  /**
   * Helper
   */
  container.register({
    scheduleRenewZeroSslCertificate: asFunction(scheduleRenewZeroSslCertificateFactory).singleton(),
  });

  return container;
}

module.exports = createDIContainer;
