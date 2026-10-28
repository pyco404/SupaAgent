/** Local presence/format check only. Does not load dotenv files or call Railway. */
export function preflight(env) {
  const failures = [];
  for (const name of ['SUPA_SITE_ORIGIN','SUPA_STAGING_USERNAME','SUPA_STAGING_PASSWORD','SUPA_WEB_USERNAME','SUPA_WEB_PASSWORD','SUPA_API_TOKEN']) {
    if (!env[name]) failures.push(`${name}: required`);
  }
  for (const name of ['SUPA_STAGING_PASSWORD','SUPA_WEB_PASSWORD']) {
    if (env[name] && env[name].length < 24) failures.push(`${name}: minimum 24 characters`);
  }
  if (env.SUPA_API_TOKEN && (env.SUPA_API_TOKEN.length < 32 || env.SUPA_API_TOKEN.length > 256 || !/^[\x21-\x7e]+$/.test(env.SUPA_API_TOKEN))) failures.push('SUPA_API_TOKEN: invalid format');
  if (env.SUPA_SITE_ORIGIN) {
    try {
      const url = new URL(env.SUPA_SITE_ORIGIN);
      if (url.protocol !== 'https:' || url.origin !== env.SUPA_SITE_ORIGIN || url.username || url.password) throw new Error();
    } catch { failures.push('SUPA_SITE_ORIGIN: exact HTTPS origin required'); }
  }
  if (env.SUPA_STAGING_PASSWORD && env.SUPA_STAGING_PASSWORD === env.SUPA_WEB_PASSWORD) failures.push('Viewer and operator passwords must differ');
  if (env.SUPA_STAGING_USERNAME && env.SUPA_STAGING_USERNAME === env.SUPA_WEB_USERNAME) failures.push('Viewer and operator usernames must differ');
  return { ready: failures.length === 0, failures, network_requests: 0, deployment_performed: false };
}
if (import.meta.url === `file://${process.argv[1]}`) {
  const result = preflight(process.env);
  console.log(JSON.stringify(result));
  process.exitCode = result.ready ? 0 : 1;
}
