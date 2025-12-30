#!/usr/bin/env node
/**
 * Headless browser test for WASM PIR client
 * 
 * Usage: node headless-test.mjs [server_url] [index]
 */

import { chromium } from 'playwright';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { createServer } from 'http';
import { readFileSync, existsSync } from 'fs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const pkgDir = join(__dirname, '..', 'pkg');

const SERVER_URL = process.argv[2] || 'http://104.204.142.13:3001';
const QUERY_INDEX = process.argv[3] || '42';

// Simple static file server
function startStaticServer(port) {
  const mimeTypes = {
    '.html': 'text/html',
    '.js': 'application/javascript',
    '.wasm': 'application/wasm',
    '.json': 'application/json',
  };

  return new Promise((resolve) => {
    const server = createServer((req, res) => {
      let filePath = join(__dirname, '..', req.url === '/' ? '/examples/index.html' : req.url);
      
      if (!existsSync(filePath)) {
        res.writeHead(404);
        res.end('Not found');
        return;
      }

      const ext = filePath.slice(filePath.lastIndexOf('.'));
      const contentType = mimeTypes[ext] || 'application/octet-stream';
      
      res.writeHead(200, { 'Content-Type': contentType });
      res.end(readFileSync(filePath));
    });

    server.listen(port, () => {
      console.log(`Static server on http://localhost:${port}`);
      resolve(server);
    });
  });
}

async function runTest() {
  console.log(`Testing PIR query against ${SERVER_URL}, index ${QUERY_INDEX}`);
  
  // Start local server for WASM files
  const staticServer = await startStaticServer(8765);
  
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  
  // Collect console logs
  const logs = [];
  page.on('console', msg => {
    logs.push(msg.text());
    console.log('  [browser]', msg.text());
  });

  try {
    // Navigate to test page
    await page.goto('http://localhost:8765/examples/index.html');
    
    // Set server URL
    await page.fill('#serverUrl', SERVER_URL);
    await page.fill('#lane', 'hot');
    
    // Initialize client
    console.log('Initializing WASM client...');
    await page.click('#initBtn');
    
    // Wait for initialization (CRS download can take a while - 81MB)
    await page.waitForFunction(() => {
      const log = document.getElementById('log');
      return log && log.textContent.includes('Client ready!');
    }, { timeout: 120000 });
    
    console.log('Client initialized!');
    
    // Set query index
    await page.fill('#index', QUERY_INDEX);
    
    // Run binary query (faster)
    console.log(`Running PIR query for index ${QUERY_INDEX}...`);
    const startTime = Date.now();
    await page.click('#queryBinaryBtn');
    
    // Wait for result (PIR queries can take 30-60 seconds for large DBs)
    await page.waitForFunction(() => {
      const log = document.getElementById('log');
      return log && (log.textContent.includes('completed') || log.textContent.includes('error'));
    }, { timeout: 120000 });
    
    const elapsed = Date.now() - startTime;
    
    // Extract result from log
    const logContent = await page.$eval('#log', el => el.textContent);
    const resultMatch = logContent.match(/Result \((\d+) bytes\): ([a-f0-9]+)/);
    
    if (resultMatch) {
      console.log(`\n[OK] PIR Query successful!`);
      console.log(`  Index: ${QUERY_INDEX}`);
      console.log(`  Result: ${resultMatch[2]}`);
      console.log(`  Size: ${resultMatch[1]} bytes`);
      console.log(`  Time: ${elapsed}ms`);
    } else if (logContent.includes('Query error')) {
      console.log(`\n[FAIL] Query failed`);
      console.log(logContent);
    }
    
  } catch (err) {
    console.error('Test failed:', err.message);
    console.log('Logs:', logs.join('\n'));
  } finally {
    await browser.close();
    staticServer.close();
  }
}

runTest().catch(console.error);
