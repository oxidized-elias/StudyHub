import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export async function checkHealth(environment = process.env, fetcher = fetch) {
  const headers = {};
  if (environment.STUDYHUB_PASSWORD) {
    headers.Authorization = `Basic ${Buffer.from(`${environment.STUDYHUB_USER || 'admin'}:${environment.STUDYHUB_PASSWORD}`).toString('base64')}`;
  }
  const response = await fetcher('http://127.0.0.1:8082/api/health', {
    headers, signal: AbortSignal.timeout(5000), redirect: 'error',
  });
  if (!response.ok) return false;
  const result = await response.json();
  return result.service === 'studyhub' && result.ok === true && result.notes === true && result.learning === true;
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  checkHealth().then(healthy => {
    if (!healthy) { console.error('StudyHub is not healthy.'); process.exitCode = 1; }
  }).catch(() => { console.error('StudyHub health endpoint is unavailable.'); process.exitCode = 1; });
}
