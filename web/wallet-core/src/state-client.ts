/**
 * EIP-7864 State Client - Query Ethereum storage slots via PIR
 *
 * Uses stem index for O(log N) lookup of PIR database indices.
 * Each storage slot query is a single PIR request.
 */

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
 * Wrapper for WASM StemIndex with TypeScript-friendly API
 */
export class StemIndexWrapper {
  private index: InstanceType<typeof import('inspire-client-wasm').StemIndex>;

  constructor(index: InstanceType<typeof import('inspire-client-wasm').StemIndex>) {
    this.index = index;
  }

  /**
   * Create a StemIndex from binary data (stem-index.bin format)
   */
  static async fromBytes(data: Uint8Array): Promise<StemIndexWrapper> {
    const wasm = await ensureWasmLoaded();
    const index = new wasm.StemIndex(data);
    return new StemIndexWrapper(index);
  }

  /**
   * Number of unique stems in the index
   */
  get count(): number {
    return this.index.count;
  }

  /**
   * Look up PIR database index for a storage slot
   *
   * @param address - Contract address (20 bytes or hex string)
   * @param slot - Storage slot (32 bytes or hex string)
   * @returns PIR database index, or null if not found
   */
  lookupStorage(address: Uint8Array | string, slot: Uint8Array | string): bigint | null {
    const addrBytes = toBytes(address, 20);
    const slotBytes = toBytes(slot, 32);

    const result = this.index.lookup_storage(addrBytes, slotBytes);
    return result >= 0 ? BigInt(result) : null;
  }

  /**
   * Look up PIR database index for basic_data (nonce, balance, code_size)
   */
  lookupBasicData(address: Uint8Array | string): bigint | null {
    const addrBytes = toBytes(address, 20);
    const result = this.index.lookup_basic_data(addrBytes);
    return result >= 0 ? BigInt(result) : null;
  }

  /**
   * Look up PIR database index for code_hash
   */
  lookupCodeHash(address: Uint8Array | string): bigint | null {
    const addrBytes = toBytes(address, 20);
    const result = this.index.lookup_code_hash(addrBytes);
    return result >= 0 ? BigInt(result) : null;
  }

  dispose(): void {
    this.index.free();
  }
}

/**
 * EIP-7864 State Client for private Ethereum state queries
 *
 * Queries individual storage slots using stem index + PIR.
 * Each query is a single PIR request (~10-50 KB depending on DB size).
 */
export class StateClient {
  private client: InstanceType<typeof import('inspire-client-wasm').PirClient> | null = null;
  private stemIndex: StemIndexWrapper | null = null;
  private serverUrl: string;
  private lane: string;

  constructor(serverUrl: string, lane: string = 'state') {
    this.serverUrl = serverUrl;
    this.lane = lane;
  }

  /**
   * Initialize the client by loading CRS and stem index
   */
  async init(): Promise<void> {
    const wasm = await ensureWasmLoaded();

    // Initialize PIR client (downloads CRS)
    this.client = new wasm.PirClient(this.serverUrl);
    await this.client.init(this.lane);

    // Download and parse stem index
    const stemRes = await fetch(`${this.serverUrl}/index/stems`);
    if (!stemRes.ok) {
      throw new Error(`Failed to fetch stem index: ${stemRes.status}`);
    }
    const stemData = new Uint8Array(await stemRes.arrayBuffer());
    this.stemIndex = await StemIndexWrapper.fromBytes(stemData);

    console.log(`StateClient initialized: ${this.stemIndex.count} stems`);
  }

  /**
   * Query a storage slot value
   *
   * @param contractAddress - Contract address (20 bytes or hex string)
   * @param storageSlot - Storage slot key (32 bytes or hex string)
   * @returns 32-byte storage value, or null if not found
   */
  async queryStorageSlot(
    contractAddress: Uint8Array | string,
    storageSlot: Uint8Array | string
  ): Promise<Uint8Array | null> {
    if (!this.client || !this.stemIndex) {
      throw new Error('Client not initialized');
    }

    // Look up PIR index using stem index
    const pirIndex = this.stemIndex.lookupStorage(contractAddress, storageSlot);
    if (pirIndex === null) {
      return null; // Slot not in database
    }

    // Execute PIR query
    const result = await this.client.query_binary(pirIndex);

    // Extract value from entry (last 32 bytes of 84-byte entry)
    if (result.length >= 84) {
      return result.slice(52, 84);
    } else if (result.length === 32) {
      return result;
    } else {
      throw new Error(`Unexpected entry size: ${result.length}`);
    }
  }

  /**
   * Query an ERC-20 token balance
   *
   * @param tokenAddress - Token contract address
   * @param walletAddress - Wallet address to query balance for
   * @param balanceSlot - Base slot for balances mapping (e.g., 9 for USDC)
   * @returns Token balance as bigint, or null if not found
   */
  async queryTokenBalance(
    tokenAddress: Uint8Array | string,
    walletAddress: Uint8Array | string,
    balanceSlot: number
  ): Promise<bigint | null> {
    const wasm = await ensureWasmLoaded();

    // Compute storage slot for balances[walletAddress]
    const walletBytes = toBytes(walletAddress, 20);
    const storageSlot = wasm.compute_balance_slot(walletBytes, balanceSlot);

    // Query the slot
    const value = await this.queryStorageSlot(tokenAddress, storageSlot);
    if (!value) return null;

    return bytesToBigInt(value);
  }

  /**
   * Query USDC balance on Sepolia
   */
  async querySepoliaUsdcBalance(walletAddress: Uint8Array | string): Promise<bigint | null> {
    const wasm = await ensureWasmLoaded();
    const usdc = wasm.sepolia_usdc();

    return this.queryTokenBalance(
      hexToBytes(usdc.address_hex),
      walletAddress,
      usdc.balance_slot
    );
  }

  /**
   * Query USDC balance on Mainnet
   */
  async queryMainnetUsdcBalance(walletAddress: Uint8Array | string): Promise<bigint | null> {
    const wasm = await ensureWasmLoaded();
    const usdc = wasm.mainnet_usdc();

    return this.queryTokenBalance(
      hexToBytes(usdc.address_hex),
      walletAddress,
      usdc.balance_slot
    );
  }

  /**
   * Get number of stems in the index (for debugging)
   */
  get stemCount(): number {
    return this.stemIndex?.count ?? 0;
  }

  dispose(): void {
    if (this.client) {
      this.client.free();
      this.client = null;
    }
    if (this.stemIndex) {
      this.stemIndex.dispose();
      this.stemIndex = null;
    }
  }
}

// Helper functions

function toBytes(input: Uint8Array | string, expectedLength: number): Uint8Array {
  if (input instanceof Uint8Array) {
    if (input.length !== expectedLength) {
      throw new Error(`Expected ${expectedLength} bytes, got ${input.length}`);
    }
    return input;
  }

  return hexToBytes(input, expectedLength);
}

function hexToBytes(hex: string, expectedLength?: number): Uint8Array {
  const cleanHex = hex.startsWith('0x') ? hex.slice(2) : hex;
  if (cleanHex.length % 2 !== 0) {
    throw new Error('Invalid hex string length');
  }

  const bytes = new Uint8Array(cleanHex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(cleanHex.slice(i * 2, i * 2 + 2), 16);
  }

  if (expectedLength !== undefined && bytes.length !== expectedLength) {
    throw new Error(`Expected ${expectedLength} bytes, got ${bytes.length}`);
  }

  return bytes;
}

function bytesToBigInt(bytes: Uint8Array): bigint {
  let result = 0n;
  for (const byte of bytes) {
    result = (result << 8n) | BigInt(byte);
  }
  return result;
}

/**
 * Format token balance with decimals
 */
export function formatTokenBalance(balance: bigint, decimals: number): string {
  const divisor = 10n ** BigInt(decimals);
  const whole = balance / divisor;
  const fraction = balance % divisor;
  const fractionStr = fraction.toString().padStart(decimals, '0');
  return `${whole}.${fractionStr}`;
}

/**
 * Format USDC balance (6 decimals)
 */
export function formatUsdc(balance: bigint): string {
  return formatTokenBalance(balance, 6);
}
