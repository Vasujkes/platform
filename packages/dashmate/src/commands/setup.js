const { Listr } = require('listr2');

const { Flags } = require('@oclif/core');

const BaseCommand = require('../oclif/command/BaseCommand');

const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

const {
  PRESET_LOCAL,
  PRESETS,
  NODE_TYPES,
  NODE_TYPE_MASTERNODE,
  MASTERNODE_DASH_AMOUNT,
  SSL_PROVIDERS,
} = require('../constants');

class SetupCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {generateBlsKeys} generateBlsKeys
   * @param {setupLocalPresetTask} setupLocalPresetTask
   * @param {setupRegularPresetTask} setupRegularPresetTask
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      preset,
      'node-type': nodeType,
    },
    {
      'external-ip': externalIp,
      'operator-bls-private-key': operatorBlsPrivateKey,
      'funding-private-key': fundingPrivateKeyString,
      'node-count': nodeCount,
      'debug-logs': debugLogs,
      'miner-interval': minerInterval,
      verbose: isVerbose,
      'ssl-provider': certificateProvider,
      'zerossl-apikey': zeroSslApiKey,
      'ssl-certificate-file': sslCertificateFilePath,
      'ssl-privatekey-file': sslCertificatePrivateKeyFilePath,
    },
    generateBlsKeys,
    setupLocalPresetTask,
    setupRegularPresetTask,
  ) {
    if (preset === PRESET_LOCAL) {
      if (nodeType === undefined) {
        // eslint-disable-next-line no-param-reassign
        nodeType = 'masternode';
      }

      if (nodeType !== NODE_TYPE_MASTERNODE) {
        throw new Error('Local development preset uses only masternode type of node');
      }
    }

    if (nodeCount !== null && (nodeCount < 3)) {
      throw new Error('node-count flag should be not less than 3');
    }

    const tasks = new Listr([
      {
        title: 'Set configuration preset',
        task: async (ctx, task) => {
          if (ctx.preset === undefined) {
            ctx.preset = await task.prompt([
              {
                type: 'select',
                message: 'Select configuration preset',
                choices: PRESETS,
                initial: 'testnet',
              },
            ]);
          }
        },
      },
      {
        task: (ctx) => {
          if (ctx.preset === PRESET_LOCAL) {
            return setupLocalPresetTask();
          }

          return setupRegularPresetTask();
        },
      },
    ],
    {
      renderer: isVerbose ? 'verbose' : 'default',
      rendererOptions: {
        showTimer: isVerbose,
        clearOutput: false,
        collapse: false,
        showSubtasks: true,
      },
    });

    try {
      await tasks.run({
        preset,
        nodeType,
        nodeCount,
        debugLogs,
        minerInterval,
        externalIp,
        operatorBlsPrivateKey,
        fundingPrivateKeyString,
        isVerbose,
        zeroSslApiKey,
        certificateProvider,
        sslCertificateFilePath,
        sslCertificatePrivateKeyFilePath,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

SetupCommand.description = `Set up node config

Set up node config
`;

SetupCommand.args = [{
  name: 'preset',
  required: false,
  description: 'Node configuration preset',
  options: PRESETS,
},
{
  name: 'node-type',
  required: false,
  description: 'Node type',
  options: NODE_TYPES,
}];

SetupCommand.flags = {
  'debug-logs': Flags.boolean({ char: 'd', description: 'enable debug logs', allowNo: true }),
  'external-ip': Flags.string({ char: 'i', description: 'external ip' }),
  'operator-bls-private-key': Flags.string({ char: 'k', description: 'operator bls private key' }),
  'funding-private-key': Flags.string({ char: 'p', description: `private key with more than ${MASTERNODE_DASH_AMOUNT} dash for funding collateral` }),
  'node-count': Flags.integer({ description: 'number of nodes to setup' }),
  'miner-interval': Flags.string({ char: 'm', description: 'interval between blocks' }),
  'ssl-provider': Flags.string({ char: 's', description: '', options: SSL_PROVIDERS.filter((item) => item !== 'selfSigned') }),
  'zerossl-apikey': Flags.string({ char: 'z', description: 'ZeroSSL API key', dependsOn: ['ssl-provider'] }),
  'ssl-certificate-file': Flags.string({ char: 'c', description: 'SSL certificate file path', dependsOn: ['ssl-provider'] }),
  'ssl-privatekey-file': Flags.string({ char: 'l', description: 'SSL certificate private key file path', dependsOn: ['ssl-provider'] }),

  verbose: Flags.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),
};

module.exports = SetupCommand;
