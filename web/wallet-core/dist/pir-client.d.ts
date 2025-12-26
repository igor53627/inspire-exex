import type { BalanceMetadata, BucketRange } from './types.js';
/**
 * WASM BucketIndex wrapper for TypeScript
 */
export declare class BucketIndexWrapper {
    private index;
    constructor(index: InstanceType<typeof import('inspire-client-wasm').BucketIndex>);
    get totalEntries(): bigint;
    /**
     * Look up bucket range for an (address, slot) pair
     */
    lookup(address: Uint8Array, slot: Uint8Array): BucketRange;
    /**
     * Apply a delta update from websocket
     * @returns Block number the delta applies to
     */
    applyDelta(data: Uint8Array): bigint;
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
     * Fetch the bucket index for sparse lookups (~150 KB download)
     */
    fetchBucketIndex(): Promise<BucketIndexWrapper>;
    dispose(): void;
}
//# sourceMappingURL=pir-client.d.ts.map