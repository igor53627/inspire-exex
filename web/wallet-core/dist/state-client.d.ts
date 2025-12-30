/**
 * EIP-7864 State Client - Query Ethereum storage slots via PIR
 *
 * Uses stem index for O(log N) lookup of PIR database indices.
 * Each storage slot query is a single PIR request.
 */
/**
 * Wrapper for WASM StemIndex with TypeScript-friendly API
 */
export declare class StemIndexWrapper {
    private index;
    constructor(index: InstanceType<typeof import('inspire-client-wasm').StemIndex>);
    /**
     * Create a StemIndex from binary data (stem-index.bin format)
     */
    static fromBytes(data: Uint8Array): Promise<StemIndexWrapper>;
    /**
     * Number of unique stems in the index
     */
    get count(): number;
    /**
     * Look up PIR database index for a storage slot
     *
     * @param address - Contract address (20 bytes or hex string)
     * @param slot - Storage slot (32 bytes or hex string)
     * @returns PIR database index, or null if not found
     */
    lookupStorage(address: Uint8Array | string, slot: Uint8Array | string): bigint | null;
    /**
     * Look up PIR database index for basic_data (nonce, balance, code_size)
     */
    lookupBasicData(address: Uint8Array | string): bigint | null;
    /**
     * Look up PIR database index for code_hash
     */
    lookupCodeHash(address: Uint8Array | string): bigint | null;
    dispose(): void;
}
/**
 * EIP-7864 State Client for private Ethereum state queries
 *
 * Queries individual storage slots using stem index + PIR.
 * Each query is a single PIR request (~10-50 KB depending on DB size).
 */
export declare class StateClient {
    private client;
    private stemIndex;
    private serverUrl;
    private lane;
    constructor(serverUrl: string, lane?: string);
    /**
     * Initialize the client by loading CRS and stem index
     */
    init(): Promise<void>;
    /**
     * Query a storage slot value
     *
     * @param contractAddress - Contract address (20 bytes or hex string)
     * @param storageSlot - Storage slot key (32 bytes or hex string)
     * @returns 32-byte storage value, or null if not found
     */
    queryStorageSlot(contractAddress: Uint8Array | string, storageSlot: Uint8Array | string): Promise<Uint8Array | null>;
    /**
     * Query an ERC-20 token balance
     *
     * @param tokenAddress - Token contract address
     * @param walletAddress - Wallet address to query balance for
     * @param balanceSlot - Base slot for balances mapping (e.g., 9 for USDC)
     * @returns Token balance as bigint, or null if not found
     */
    queryTokenBalance(tokenAddress: Uint8Array | string, walletAddress: Uint8Array | string, balanceSlot: number): Promise<bigint | null>;
    /**
     * Query USDC balance on Sepolia
     */
    querySepoliaUsdcBalance(walletAddress: Uint8Array | string): Promise<bigint | null>;
    /**
     * Query USDC balance on Mainnet
     */
    queryMainnetUsdcBalance(walletAddress: Uint8Array | string): Promise<bigint | null>;
    /**
     * Get number of stems in the index (for debugging)
     */
    get stemCount(): number;
    dispose(): void;
}
/**
 * Format token balance with decimals
 */
export declare function formatTokenBalance(balance: bigint, decimals: number): string;
/**
 * Format USDC balance (6 decimals)
 */
export declare function formatUsdc(balance: bigint): string;
//# sourceMappingURL=state-client.d.ts.map