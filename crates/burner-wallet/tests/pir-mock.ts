/**
 * PIR Server Mock for E2E Testing
 * 
 * Intercepts PIR server requests at the Playwright level to simulate
 * a PIR server without needing real RLWE crypto infrastructure.
 * 
 * This allows testing:
 * - PIR connection flow
 * - Metadata display
 * - Balance query UI (with mock data)
 * - Server log visualization
 */

import { Page, Route } from '@playwright/test';

export interface MockPirConfig {
  serverUrl: string;
  addresses: string[];
  balances: Map<string, { eth: bigint; usdc: bigint }>;
  snapshotBlock: number;
  blockHash: string;
}

export const DEFAULT_MOCK_CONFIG: MockPirConfig = {
  serverUrl: 'http://localhost:3001',
  addresses: [
    '0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045', // vitalik.eth
    '0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266', // hardhat account 0
    '0x70997970C51812dc3A010C7d01b50e0d17dc79C8', // hardhat account 1
  ],
  balances: new Map([
    ['0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045', { eth: 1000n * 10n ** 18n, usdc: 50000n * 10n ** 6n }],
    ['0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266', { eth: 10000n * 10n ** 18n, usdc: 100000n * 10n ** 6n }],
    ['0x70997970C51812dc3A010C7d01b50e0d17dc79C8', { eth: 5000n * 10n ** 18n, usdc: 25000n * 10n ** 6n }],
  ]),
  snapshotBlock: 7500000,
  blockHash: '0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef',
};

/**
 * Mock CRS response - minimal valid structure that won't cause parse errors
 * Note: The actual crypto won't work, but the init flow will proceed
 */
function mockCrsResponse(entryCount: number) {
  return {
    crs: JSON.stringify({
      params: {
        n: 2048,
        q: "0x7fffffffe0001",
        sigma: 3.2,
        logq: 51,
      },
      a_polynomials: [],
    }),
    lane: 'hot',
    entry_count: entryCount,
    shard_config: {
      shard_count: 1,
      entries_per_shard: entryCount,
    },
  };
}

/**
 * Mock metadata response for balance queries
 */
function mockMetadataResponse(config: MockPirConfig) {
  return {
    addresses: config.addresses.map(a => a.toLowerCase()),
    snapshotBlock: config.snapshotBlock,
    blockHash: config.blockHash,
    entryCount: config.addresses.length,
    lane: 'balances',
  };
}

/**
 * Mock PIR query response - returns fake encrypted response
 * The client can't decrypt this, so we need a different approach
 */
function mockQueryBinaryResponse(): ArrayBuffer {
  // Return 64 bytes: 32 for ETH balance, 32 for USDC balance
  // This is what the client expects to extract
  const buffer = new ArrayBuffer(1024);
  const view = new Uint8Array(buffer);
  // Fill with zeros - client will get garbage but won't crash
  view.fill(0);
  return buffer;
}

/**
 * Setup mock routes for PIR server endpoints
 */
export async function setupPirMock(page: Page, config: MockPirConfig = DEFAULT_MOCK_CONFIG) {
  const pirUrlPattern = new RegExp(`^${config.serverUrl.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`);

  await page.route(pirUrlPattern, async (route: Route) => {
    const url = route.request().url();
    const method = route.request().method();

    console.log(`[PIR Mock] ${method} ${url}`);

    // GET /crs/{lane}
    if (url.includes('/crs/') && method === 'GET') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(mockCrsResponse(config.addresses.length)),
      });
      return;
    }

    // GET /metadata/balances
    if (url.includes('/metadata/balances') && method === 'GET') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(mockMetadataResponse(config)),
      });
      return;
    }

    // POST /query/{lane}/seeded/binary
    if (url.includes('/query/') && url.includes('/seeded/binary') && method === 'POST') {
      await route.fulfill({
        status: 200,
        contentType: 'application/octet-stream',
        body: Buffer.from(mockQueryBinaryResponse()),
      });
      return;
    }

    // GET /health
    if (url.includes('/health') && method === 'GET') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          status: 'ready',
          lanes: {
            hot_entries: config.addresses.length,
            cold_entries: 0,
            hot_contracts: config.addresses.length,
            block_number: config.snapshotBlock,
          },
        }),
      });
      return;
    }

    // GET /info
    if (url.includes('/info') && method === 'GET') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          version: '0.1.0-mock',
          config_hash: 'mock-hash',
          manifest_block: config.snapshotBlock,
          hot_entries: config.addresses.length,
          cold_entries: 0,
          hot_contracts: config.addresses.length,
          block_number: config.snapshotBlock,
        }),
      });
      return;
    }

    // Fallback - return 404
    console.log(`[PIR Mock] Unhandled route: ${method} ${url}`);
    await route.fulfill({
      status: 404,
      body: 'Not found (PIR mock)',
    });
  });
}

/**
 * Alternative: Mock that works with the actual wallet JavaScript
 * by injecting a fake PIR client into the page
 */
export async function injectFakePirClient(page: Page, config: MockPirConfig = DEFAULT_MOCK_CONFIG) {
  await page.addInitScript((cfg) => {
    (window as any).__PIR_MOCK_CONFIG__ = cfg;
    (window as any).__PIR_MOCK_ENABLED__ = true;
  }, {
    addresses: config.addresses,
    snapshotBlock: config.snapshotBlock,
    blockHash: config.blockHash,
    balances: Object.fromEntries(
      Array.from(config.balances.entries()).map(([k, v]) => [
        k.toLowerCase(),
        { eth: v.eth.toString(), usdc: v.usdc.toString() }
      ])
    ),
  });
}

/**
 * Create a custom config for testing specific addresses
 */
export function createMockConfig(overrides: Partial<MockPirConfig>): MockPirConfig {
  return {
    ...DEFAULT_MOCK_CONFIG,
    ...overrides,
  };
}
