import type { BalanceMetadata, BucketRange, RangeDeltaInfoResponse, RangeSyncResult } from './types.js';
/**
 * WASM BucketIndex wrapper for TypeScript
 */
export declare class BucketIndexWrapper {
    private index;
    private _blockNumber;
    constructor(index: InstanceType<typeof import('inspire-client-wasm').BucketIndex>, blockNumber?: bigint);
    get totalEntries(): bigint;
    get blockNumber(): bigint;
    /**
     * Look up bucket range for an (address, slot) pair
     */
    lookup(address: Uint8Array, slot: Uint8Array): BucketRange;
    /**
     * Apply a delta update from websocket
     * @returns Block number the delta applies to
     */
    applyDelta(data: Uint8Array): bigint;
    /**
     * Apply a range delta (from /index/deltas endpoint)
     * @returns Block number after applying the delta
     */
    applyRangeDelta(data: Uint8Array): bigint;
    dispose(): void;
}
export declare class PirBalanceClient {
    private client;
    private metadata;
    private serverUrl;
    private lane;
    constructor(serverUrl: string, lane?: string);
    init(): Promise<void>;
    getMetadata(): BalanceMetadata | null;
    getSnapshotBlock(): bigint;
    getSnapshotBlockHash(): string;
    findAddressIndex(address: string): number;
    queryBalance(address: string): Promise<{
        eth: bigint;
        usdc: bigint;
    } | null>;
    /**
     * Fetch the bucket index for sparse lookups (~512 KB uncompressed from /index/raw)
     */
    fetchBucketIndex(): Promise<BucketIndexWrapper>;
    /**
     * Fetch range delta info for efficient sync
     */
    fetchRangeDeltaInfo(): Promise<RangeDeltaInfoResponse>;
    /**
     * Sync bucket index using range-based delta
     *
     * Downloads only the smallest range covering the sync gap, then applies it.
     * Much more efficient than re-downloading the full 256KB index.
     *
     * @param bucketIndex - Existing bucket index to update
     * @returns Sync result with block number and bytes downloaded, or null if already synced
     */
    syncBucketIndex(bucketIndex: BucketIndexWrapper): Promise<RangeSyncResult | null>;
    dispose(): void;
}
//# sourceMappingURL=pir-client.d.ts.map