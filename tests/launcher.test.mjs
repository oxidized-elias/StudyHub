import test from 'node:test';
import assert from 'node:assert/strict';
import { configuration, main } from '../server.mjs';

test('launcher builds only the integrated binary with paths independent of current directory', () => {
  const c = configuration(['--release', '--offline'], { DATA_DIR: '/tmp/studyhub-test-data' });
  assert.equal(c.env.DATA_DIR, '/tmp/studyhub-test-data');
  assert.equal(c.env.LEARNING_DATA_DIR, '/tmp/studyhub-test-data/learning');
  assert.equal(c.env.VAULT_DIR, '/tmp/studyhub-test-data/vault');
  assert.ok(c.binary.endsWith('/release/study-hub'));
  assert.deepEqual(c.cargo.slice(-4), ['-p', 'study-hub', '--release', '--offline']);
});
test('invalid flags fail before builds or data creation', () => {
  assert.throws(() => configuration(['--connect']), /Unknown option/);
  assert.throws(() => configuration(['--no-build', '--build-only']), /cannot be combined/);
});
test('help works without Cargo or an existing database', async () => {
  const log = console.log;
  let output = '';
  console.log = message => { output += message; };
  try { assert.equal(await main(['--help']), 0); }
  finally { console.log = log; }
  assert.match(output, /One application, one server/);
});
