/** Local presence/format check only. Does not load dotenv files or call Railway. */
export function preflight(env) {
  const failures = [];
  for (const name of ['SUPA_SITE_ORIGIN','SUPA_STAGING_USERNAME','SUPA_STAGING_PASSWORD','SUPA_WEB_OPERATORS','SUPA_OPERATOR_CREDENTIALS','SUPA_API_TOKEN']) {
    if (!env[name]) failures.push(`${name}: required`);
  }
  const validToken = value => typeof value === 'string' && value.length >= 32 && value.length <= 256 && /^[\x21-\x7e]+$/.test(value);
  if (env.SUPA_STAGING_PASSWORD && env.SUPA_STAGING_PASSWORD.length < 24) failures.push('SUPA_STAGING_PASSWORD: minimum 24 characters');
  if (env.SUPA_API_TOKEN && !validToken(env.SUPA_API_TOKEN)) failures.push('SUPA_API_TOKEN: invalid format');
  if (env.SUPA_WEB_USERNAME || env.SUPA_WEB_PASSWORD || env.SUPA_OPERATOR_ID) failures.push('Legacy shared operator configuration is forbidden in staging');
  let credentials = [], operators = [];
  for (const [name, assign] of [['SUPA_OPERATOR_CREDENTIALS', value => { credentials = value; }], ['SUPA_WEB_OPERATORS', value => { operators = value; }]]) {
    if (!env[name]) continue;
    try {
      const value = JSON.parse(env[name]);
      if (!Array.isArray(value) || value.length < 1 || value.length > 100) throw Error();
      assign(value);
    } catch { failures.push(`${name}: bounded nonempty JSON array required`); }
  }
  const ids = new Set(), tokens = new Set();
  for (const credential of credentials) {
    if (!credential || Object.keys(credential).sort().join(',') !== 'id,role,token' ||
        typeof credential.id !== 'string' || !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(credential.id) || /^0{8}-0{4}-0{4}-0{4}-0{12}$/.test(credential.id) ||
        !['viewer','operator','owner'].includes(credential.role) || !validToken(credential.token) ||
        credential.token === env.SUPA_API_TOKEN || ids.has(credential.id.toLowerCase()) || tokens.has(credential.token)) {
      failures.push('SUPA_OPERATOR_CREDENTIALS: invalid or ambiguous principal mapping');
      continue;
    }
    ids.add(credential.id.toLowerCase()); tokens.add(credential.token);
  }
  const usernames = new Set(), webTokens = new Set();
  for (const account of operators) {
    if (!account || Object.keys(account).sort().join(',') !== 'api_token,password,username' ||
        typeof account.username !== 'string' || !account.username.length || account.username.length > 80 || /[:\x00-\x20\x7f]/.test(account.username) ||
        typeof account.password !== 'string' || account.password.length < 24 || account.password.length > 512 ||
        account.username === env.SUPA_STAGING_USERNAME || account.password === env.SUPA_STAGING_PASSWORD ||
        !tokens.has(account.api_token) || usernames.has(account.username) || webTokens.has(account.api_token)) {
      failures.push('SUPA_WEB_OPERATORS: invalid, shared, or unbound operator account');
      continue;
    }
    usernames.add(account.username); webTokens.add(account.api_token);
  }
  if (env.SUPA_SITE_ORIGIN) {
    try {
      const url = new URL(env.SUPA_SITE_ORIGIN);
      if (url.protocol !== 'https:' || url.origin !== env.SUPA_SITE_ORIGIN || url.username || url.password) throw new Error();
    } catch { failures.push('SUPA_SITE_ORIGIN: exact HTTPS origin required'); }
  }
  return { ready: failures.length === 0, failures, network_requests: 0, deployment_performed: false };
}
if (import.meta.url === `file://${process.argv[1]}`) {
  const result = preflight(process.env);
  console.log(JSON.stringify(result));
  process.exitCode = result.ready ? 0 : 1;
}
