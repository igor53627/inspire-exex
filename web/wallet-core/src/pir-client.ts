import type { BalanceMetadata, BucketRange, BucketIndexInfo } from './types.js';

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

  constructor(index: InstanceType<typeof import('inspire-client-wasm').BucketIndex>) {
    this.index = index;
  }

  get totalEntries(): bigint {
    return BigInt(this.index.total_entries);
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
    return BigInt(this.index.apply_delta(data));
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
