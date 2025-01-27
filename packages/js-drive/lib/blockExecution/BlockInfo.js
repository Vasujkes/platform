class BlockInfo {
  /**
   * @type {number}
   */
  height;

  /**
   * @type {number}
   */
  epoch;

  /**
   * @type {number}
   */
  timeMs;

  /**
   * @param {number} height
   * @param {number} epoch
   * @param {number} timeMs
   */
  constructor(height, epoch, timeMs) {
    this.height = height;
    this.epoch = epoch;
    this.timeMs = timeMs;
  }

  /**
   * @param {BlockExecutionContext} blockExecutionContext
   * @returns {BlockInfo}
   */
  static createFromBlockExecutionContext(blockExecutionContext) {
    const epochInfo = blockExecutionContext.getEpochInfo();

    return new BlockInfo(
      blockExecutionContext.getHeight().toNumber(),
      epochInfo.currentEpochIndex,
      blockExecutionContext.getTimeMs(),
    );
  }
}

module.exports = BlockInfo;
