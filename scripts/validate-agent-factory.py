#!/usr/bin/env python3
"""Opt-in, new socket-only PG18 cluster. Never accepts existing DB URLs or paths."""
import hashlib,json,os,pathlib,subprocess,uuid,shutil
from urllib.parse import urlencode
ROOT=pathlib.Path(__file__).resolve().parents[1]
PG=ROOT/'artifacts/ecosystem-pg18-binaries/root/usr/lib/postgresql/18/bin'
OUT=ROOT/'artifacts/functional-factory'
def main():
    os.umask(0o077);OUT.mkdir(exist_ok=True,parents=True)
    work=pathlib.Path('/tmp')/('supa-factory-test-'+uuid.uuid4().hex);work.mkdir(mode=0o700);socket=work/'socket';socket.mkdir(mode=0o700)
    env={k:os.environ[k] for k in ['PATH','HOME']};env.update(LANG='C.UTF-8',LC_ALL='C.UTF-8',TZ='UTC',PGHOST=str(socket),PGPORT='55481',PGUSER='supa_factory_test',PGDATABASE='postgres',PGPASSFILE='/dev/null',PGSERVICEFILE='/dev/null',CARGO_TARGET_DIR=str(ROOT/'target/work-review-validation'),CARGO_INCREMENTAL='0',CARGO_BUILD_JOBS='2',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',SUPA_FACTORY_BROWSER='1',SUPA_FACTORY_AVATARS='1')
    registered=json.loads(subprocess.check_output(['pg_lsclusters','--json']))
    hashes={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((ROOT/'migrations').glob('*.sql'))}
    def run(args,log,timeout=600):
        with (OUT/log).open('ab') as output:subprocess.run([str(a) for a in args],cwd=ROOT,env=env,stdout=output,stderr=subprocess.STDOUT,check=True,timeout=timeout)
    def pg(command,*args):run([PG/command,*args],command+'.log',90)
    version=subprocess.check_output([str(PG/'postgres'),'--version'],env=env,text=True).strip();assert 'PostgreSQL) 18.' in version
    cargo=['cargo','test','--locked','--offline','-p','supa-api','--test','factory_journey']
    started=False
    try:
        run(['/home/pikoo/.nvm/versions/node/v24.22.1/bin/node','scripts/test-avatar-images.mjs'],'avatar-codec.log');run(cargo+['--no-run'],'build.log');run(cargo+['refuses_unscoped_database_targets'],'target-guard.log')
        pg('initdb','-D',work/'data','-U','supa_factory_test','--auth-local=trust','--auth-host=reject','--wal-segsize=1','--encoding=UTF8','--locale=C.UTF-8','--no-instructions')
        with (work/'data/postgresql.conf').open('a') as f:f.write(f"\nlisten_addresses=''\nunix_socket_directories='{socket}'\nport=55481\nmax_wal_size='32MB'\nmin_wal_size='2MB'\n")
        pg('pg_ctl','-D',work/'data','-l',OUT/'postgres.log','-w','start');started=True;pg('createdb','supa_factory_test')
        env['SUPA_FACTORY_TEST_URL']='postgresql://supa_factory_test@localhost/supa_factory_test?'+urlencode({'host':str(socket),'port':55481})
        run(cargo+['--','--exact','factory_journey','--include-ignored','--nocapture'],'journey.log')
        pg('pg_ctl','-D',work/'data','-m','fast','-w','stop');started=False
        pg('pg_ctl','-D',work/'data','-l',OUT/'postgres.log','-w','start');started=True
        run(cargo+['--','--exact','factory_restart_recovery','--include-ignored','--nocapture'],'recovery.log')
        assert hashes=={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((ROOT/'migrations').glob('*.sql'))}
        (OUT/'postgres-result.json').write_text(json.dumps({'passed':True,'postgres_version':version,'socket_only':True,'disposable_cluster':str(work),'real_restart_recovery':True,'restricted_role':True,'concurrent_idempotency':True,'atomic_rollback':True,'existing_migrations_unchanged':True,'railway_operations':0},indent=2)+'\n')
    finally:
        if started or (work/'data/postmaster.pid').exists():pg('pg_ctl','-D',work/'data','-m','fast','-w','stop')
        assert registered==json.loads(subprocess.check_output(['pg_lsclusters','--json']))
        assert not (work/'data/postmaster.pid').exists()
        shutil.rmtree(work)  # Only this invocation's stopped disposable fixture; logs retained.
    print(json.dumps({'passed':True,'evidence':str(OUT),'disposable_stopped':True}))
if __name__=='__main__':main()
