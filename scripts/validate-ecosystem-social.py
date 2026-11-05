#!/usr/bin/env python3
"""Build and test ecosystem preparation in a NEW socket-only disposable cluster.

No database URL/data-directory inputs. No network listeners, credentials, provider
calls, Railway operations or protected database connections. Retain private logs.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import uuid
from urllib.parse import urlencode

ROOT=Path(__file__).resolve().parents[1]
PG=Path('/usr/lib/postgresql/16/bin')

def registered():
    return json.loads(subprocess.check_output(['pg_lsclusters','--json'],text=True))

def protected():
    primary=ROOT/'postgres-data/infrastructure-v1/data'
    assert not (primary/'postmaster.pid').exists(),'protected_primary_must_remain_stopped'
    paths=[primary/name for name in ('global/pg_control','postgresql.conf','pg_hba.conf')]
    assert all(p.is_file() for p in paths),'protected_primary_identity_required'
    return {str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}

def main():
    os.umask(0o077)
    run=uuid.uuid4().hex
    work=Path('/tmp')/('supa-ecosystem-test-'+run)
    work.mkdir(mode=0o700)
    socket=work/'socket';socket.mkdir(mode=0o700)
    out=ROOT/'artifacts/supa-agency-ecosystem-integration'/run
    out.mkdir(mode=0o700,parents=True)
    env={k:os.environ[k] for k in ('PATH','HOME')}
    env.update(LANG='C.UTF-8',LC_ALL='C.UTF-8',TZ='UTC',PGHOST=str(socket),PGPORT='55479',
        PGUSER='supa_ecosystem_test',PGDATABASE='postgres',PGPASSFILE='/dev/null',PGSERVICEFILE='/dev/null',
        CARGO_TARGET_DIR=str(ROOT/'target/work-review-validation'),CARGO_INCREMENTAL='0',CARGO_BUILD_JOBS='2',
        CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0')
    before=registered()
    protected_before=protected()
    (out/'protected-before.json').write_text(json.dumps(protected_before,sort_keys=True)+'\n')
    (out/'registered-before.json').write_text(json.dumps(before,sort_keys=True)+'\n')
    checksums={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((ROOT/'migrations').glob('*.sql'))}
    assert len(checksums)==13
    def command(args,log,timeout=180):
        with (out/log).open('ab') as output:
            subprocess.run([str(a) for a in args],cwd=ROOT,env=env,stdout=output,stderr=subprocess.STDOUT,check=True,timeout=timeout)
    def pg(tool,*args):command([PG/tool,*args],tool+'.log')
    cargo=['cargo','test','--locked','--offline','-p','supa-persistence','--test','ecosystem_social']
    started=False
    try:
        command(cargo+['--no-run'],'build.log',600)
        command(cargo+['--','--exact','rejects_unexpected_database_transport_port_and_primary_path'],'guards.log')
        pg('initdb','-D',work/'data','-U','supa_ecosystem_test','--auth-local=trust','--auth-host=reject','--encoding=UTF8','--locale=C.UTF-8','--no-instructions')
        with (work/'data/postgresql.conf').open('a') as f:
            f.write(f"\nlisten_addresses=''\nunix_socket_directories='{socket}'\nport=55479\n")
        pg('pg_ctl','-D',work/'data','-l',out/'server.log','-w','start');started=True
        pg('createdb','supa_ecosystem_test')
        env['SUPA_ECOSYSTEM_TEST_URL']='postgresql://supa_ecosystem_test@localhost/supa_ecosystem_test?'+urlencode({'host':str(socket),'port':55479})
        command(cargo+['--','--exact','ecosystem_lifecycle','--include-ignored','--test-threads=1'],'lifecycle.log')
        pg('pg_ctl','-D',work/'data','-m','fast','-w','stop');started=False
        pg('pg_ctl','-D',work/'data','-l',out/'server.log','-w','start');started=True
        command(cargo+['--','--exact','ecosystem_recovery','--include-ignored','--test-threads=1'],'recovery.log')
        assert checksums=={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((ROOT/'migrations').glob('*.sql'))}
        result={'passed':True,'postgres_version':subprocess.check_output([str(PG/'postgres'),'--version'],text=True).strip(),
            'socket_only':True,'database':'supa_ecosystem_test','existing_migrations_unchanged':checksums,
            'extension_installed_only_in_disposable_database':True,'tests':['target_guard','authenticated_lifecycle','concurrent_deduplication','restart_recovery'],
            'synthetic_only':True,'financial_baseline_unchanged':True,'paid_calls':0,'railway_operations':0,'publishing_enabled':False}
    finally:
        if started or (work/'data/postmaster.pid').exists():pg('pg_ctl','-D',work/'data','-m','fast','-w','stop')
        after=registered()
        (out/'registered-after.json').write_text(json.dumps(after,sort_keys=True)+'\n')
        assert before==after,'registered_cluster_state_changed'
        assert protected_before==protected(),'protected_primary_changed'
    result['disposable_stopped']=True
    result['protected_primary_stopped_and_unchanged']=True
    result['registered_system_clusters_unchanged']=True
    (out/'validation.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'passed':True,'evidence':str(out.relative_to(ROOT)),'disposable_stopped':True}))

if __name__=='__main__':main()
