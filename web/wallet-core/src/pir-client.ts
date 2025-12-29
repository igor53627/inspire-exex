import type { BalanceMetadata, BucketRange, BucketIndexInfo, RangeDeltaInfoResponse, RangeSyncResult } from './types.js';

let wasmModule: typeof import('inspire-client-wasm') | null = null;
let wasmInit: Promise<void> | null = null;

async function ensureWasmLoaded(): Promise<typeof import('inspire-client-wasm')> {
  if (wasmModule) return wasmModule;
  
  if (!wasmInit) {
    wasmInit = (async () => {
      const wasm = await import('inspire-client-wasm');
      await wasm.default();
      wasmModule = wasm;
    })();
  }
  
  await wasmInit;
  return wasmModule!;
}

/**
 * WASM BucketIndex wrapper for TypeScript
 */
export class BucketIndexWrapper {
  private index: InstanceType<typeof import('inspire-client-wasm').BucketIndex>;
  private _blockNumber: bigint = 0n;

  constructor(index: InstanceType<typeof import('inspire-client-wasm').BucketIndex>, blockNumber?: bigint) {
    this.index = index;
    if (blockNumber !== undefined) {
      this._blockNumber = blockNumber;
    }
  }

  get totalEntries(): bigint {
    return BigInt(this.index.total_entries);
  }

  get blockNumber(): bigint {
    return this._blockNumber;
  }

  /**
   * Look up bucket range for an (address, slot) pair
   */
  lookup(address: Uint8Array, slot: Uint8Array): BucketRange {
    const result = this.index.lookup(address, slot);
    return {
      bucketId: BigInt(result[0]),
      startIndex: BigInt(result[1]),
      count: BigInt(result[2]),
    };
  }

  /**
   * Apply a delta update from websocket
   * @returns Block number the delta applies to
   */
  applyDelta(data: Uint8Array): bigint {
    const blockNum = BigInt(this.index.apply_delta(data));
    this._blockNumber = blockNum;
    return blockNum;
  }

  /**
   * Apply a range delta (from /index/deltas endpoint)
   * @returns Block number after applying the delta
   */
  applyRangeDelta(data: Uint8Array): bigint {
    const blockNum = BigInt(this.index.apply_range_delta(data));
    this._blockNumber = blockNum;
    return blockNum;
  }

  dispose(): void {
    this.index.free();
  }
}

export class PirBalanceClient {
  private client: InstanceType<typeof import('inspire-client-wasm').PirClient> | null = null;
  private metadata: BalanceMetadata | null = null;
  private serverUrl: string;
  private lane: string;

  constructor(serverUrl: string, lane: string = 'balances') {
    this.serverUrl = serverUrl;
    this.lane = lane;
  }

  async init(): Promise<void> {
    const wasm = await ensureWasmLoaded();
    
    const metadataRes = await fetch(`${this.serverUrl}/metadata/${this.lane}`);
    if (!metadataRes.ok) {
      throw new Error(`Failed to fetch metadata: ${metadataRes.status}`);
    }
    this.metadata = await metadataRes.json();

    this.client = new wasm.PirClient(this.serverUrl);
    await this.client.init(this.lane);
  }

  getMetadata(): BalanceMetadata | null {
    return this.metadata;
  }

  getSnapshotBlock(): bigint {
    if (!this.metadata) throw new Error('Not initialized');
    return BigInt(this.metadata.snapshotBlock);
  }

  getSnapshotBlockHash(): string {
    if (!this.metadata) throw new Error('Not initialized');
    return this.metadata.snapshotBlockHash;
  }

  findAddressIndex(address: string): number {
    if (!this.metadata) throw new Error('Not initialized');
    
    const normalized = address.toLowerCase();
    const idx = this.metadata.addresses.findIndex(
      a => a.toLowerCase() === normalized
    );
    
    return idx;
  }

  async queryBalance(address: string): Promise<{ eth: bigint; usdc: bigint } | null> {
    if (!this.client || !this.metadata) {
      throw new Error('Client not initialized');
    }

    const index = this.findAddressIndex(address);
    if (index < 0) {
      return null;
    }

    const result = await this.client.query_binary(BigInt(index));
    
    if (result.length < 64) {
      throw new Error(`Invalid balance record size: ${result.length}`);
    }

    const ethBytes = result.slice(0, 32);
    const usdcBytes = result.slice(32, 64);

    return {
      eth: bytesToBigInt(ethBytes),
      usdc: bytesToBigInt(usdcBytes),
    };
  }

  /**
   * Fetch the bucket index for sparse lookups (~512 KB uncompressed from /index/raw)
   */
  async fetchBucketIndex(): Promise<BucketIndexWrapper> {
    if (!this.client) {
      throw new Error('Client not initialized');
    }
    
    const index = await this.client.fetch_bucket_index();
    return new BucketIndexWrapper(index);
  }

  /**
   * Fetch range delta info for efficient sync
   */
  async fetchRangeDeltaInfo(): Promise<RangeDeltaInfoResponse> {
    const res = await fetch(`${this.serverUrl}/index/deltas/info`);
    if (!res.ok) {
      throw new Error(`Failed to fetch range delta info: ${res.status}`);
    }
    return res.json();
  }

  /**
   * Sync bucket index using range-based delta
   * 
   * Downloads only the smallest range covering the sync gap, then applies it.
   * Much more efficient than re-downloading the full 256KB index.
   * 
   * @param bucketIndex - Existing bucket index to update
   * @returns Sync result with block number and bytes downloaded, or null if already synced
   */
  async syncBucketIndex(bucketIndex: BucketIndexWrapper): Promise<RangeSyncResult | null> {
    const info = await this.fetchRangeDeltaInfo();
    const serverBlock = BigInt(info.current_block);
    const clientBlock = bucketIndex.blockNumber;
    
    if (serverBlock <= clientBlock) {
      return null; // Already synced
    }
    
    const behindBlocks = Number(serverBlock - clientBlock);
    
    // Find smallest range that covers our gap
    let selectedRange = -1;
    for (let i = 0; i < info.ranges.length; i++) {
      if (behindBlocks <= info.ranges[i].blocks_covered) {
        selectedRange = i;
        break;
      }
    }
    
    if (selectedRange < 0) {
      throw new Error(`Too far behind (${behindBlocks} blocks), need full index refresh`);
    }
    
    const range = info.ranges[selectedRange];
    
    // Fetch just this range using HTTP Range request
    const res = await fetch(`${this.serverUrl}/index/deltas`, {
      headers: {
        'Range': `bytes=${range.offset}-${range.offset + range.size - 1}`,
      },
    });
    
    if (!res.ok && res.status !== 206) {
      throw new Error(`Failed to fetch range delta: ${res.status}`);
    }
    
    const data = new Uint8Array(await res.arrayBuffer());
    const newBlock = bucketIndex.applyRangeDelta(data);
    
    return {
      blockNumber: newBlock,
      rangeIndex: selectedRange,
      blocksCovered: range.blocks_covered,
      bytesDownloaded: data.length,
    };
  }

  dispose(): void {
    if (this.client) {
      this.client.free();
      this.client = null;
    }
  }
}

function bytesToBigInt(bytes: Uint8Array): bigint {
  let result = 0n;
  for (const byte of bytes) {
    result = (result << 8n) | BigInt(byte);
  }
  return result;
}
