#!/usr/bin/env node
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve, join } from 'node:path';
import { existsSync, mkdirSync } from 'node:fs';
import { parseArgs } from 'node:util';

export function configuration(args, environment = process.env) {
  const { values } = parseArgs({ args, options: {
    offline: { type: 'boolean' }, release: { type: 'boolean' },
    'no-build': { type: 'boolean' }, 'build-only': { type: 'boolean' },
    help: { type: 'boolean', short: 'h' },
  }});
  if (values['no-build'] && values['build-only']) throw new Error('--no-build and --build-only cannot be combined');
  const root = dirname(fileURLToPath(import.meta.url));
  const data = resolve(root, environment.DATA_DIR || 'data');
  const env = { ...environment, DATA_DIR: data,
    LEARNING_DATA_DIR: resolve(root, environment.LEARNING_DATA_DIR || join(data, 'learning')),
    VAULT_DIR: resolve(root, environment.VAULT_DIR || join(data, 'vault')),
    CARGO_TARGET_DIR: resolve(root, environment.CARGO_TARGET_DIR || 'target'),
  };
  const profile = values.release ? 'release' : 'debug';
  const binary = join(env.CARGO_TARGET_DIR, profile, process.platform === 'win32' ? 'study-hub.exe' : 'study-hub');
  const cargo = ['build', '--locked', '--manifest-path', join(root, 'Cargo.toml'), '-p', 'study-hub'];
  if (values.release) cargo.push('--release');
  if (values.offline) cargo.push('--offline');
  return { values, root, env, binary, cargo };
}
async function execute(command, args, options) {
  const child = spawn(command, args, { ...options, stdio: 'inherit' });
  let stopping = false;
  const stop = signal => { if (!stopping) { stopping = true; child.kill(signal); } };
  const interrupt = () => stop('SIGINT');
  const terminate = () => stop('SIGTERM');
  process.on('SIGINT', interrupt);
  process.on('SIGTERM', terminate);
  try {
    return await new Promise((accept, reject) => {
      child.once('error', reject);
      child.once('exit', (code, signal) => accept(code ?? (signal === 'SIGINT' ? 130 : 143)));
    });
  } finally {
    process.off('SIGINT', interrupt);
    process.off('SIGTERM', terminate);
  }
}
export async function main(args = process.argv.slice(2)) {
  const c = configuration(args);
  if (c.values.help) {
    console.log('Usage: node server.mjs [--release] [--offline] [--no-build] [--build-only]\nOne application, one server. Default: http://127.0.0.1:8082\nSet STUDYHUB_BIND, DATA_DIR and VAULT_DIR to override defaults.');
    return 0;
  }
  const options = { cwd: c.root, env: c.env };
  if (!c.values['no-build']) {
    const code = await execute('cargo', c.cargo, options);
    if (code) return code;
  }
  if (c.values['build-only']) return 0;
  if (!existsSync(c.binary)) throw new Error('Application binary is missing. Run node server.mjs --release --build-only first.');
  mkdirSync(c.env.DATA_DIR, { recursive: true, mode: 0o700 });
  return execute(c.binary, [], options);
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().then(code => { process.exitCode = code; }).catch(error => {
    console.error(`StudyHub: ${error.message}`);
    process.exitCode = 1;
  });
}
