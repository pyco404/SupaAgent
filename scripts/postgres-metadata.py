"""Deterministic catalog comparison; keeps definitions/ACLs, never table records."""
import json

SNAPSHOT_SQL = r"""
WITH namespaces AS (
 SELECT oid,nspname,nspowner,nspacl FROM pg_namespace WHERE nspname IN ('public','epoch_guard')
), objects AS (
 -- The first UNION arm determines the identity type. `name` silently
 -- truncates qualified identities to 63 bytes and collapses JSON keys.
 SELECT 'schemas' AS kind,n.nspname::text AS identity,
   jsonb_build_object('owner',pg_get_userbyid(n.nspowner),'acl',
    (SELECT jsonb_agg(jsonb_build_array(pg_get_userbyid(a.grantor),
       CASE WHEN a.grantee=0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee) END,
       a.privilege_type,a.is_grantable) ORDER BY pg_get_userbyid(a.grantor),
       CASE WHEN a.grantee=0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee) END,a.privilege_type,a.is_grantable)
     FROM aclexplode(coalesce(n.nspacl,acldefault('n',n.nspowner))) a)) AS value
 FROM namespaces n
 UNION ALL
 SELECT 'functions',n.nspname||'.'||p.proname||'('||pg_get_function_identity_arguments(p.oid)||')',
   jsonb_build_object('definition',pg_get_functiondef(p.oid),'owner',pg_get_userbyid(p.proowner),
    'acl',(SELECT jsonb_agg(jsonb_build_array(pg_get_userbyid(a.grantor),
       CASE WHEN a.grantee=0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee) END,
       a.privilege_type,a.is_grantable) ORDER BY pg_get_userbyid(a.grantor),
       CASE WHEN a.grantee=0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee) END,a.privilege_type,a.is_grantable)
     FROM aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) a))
 FROM pg_proc p JOIN namespaces n ON n.oid=p.pronamespace WHERE p.prokind IN ('f','p','w')
 UNION ALL
 SELECT 'triggers',n.nspname||'.'||c.relname||'.'||t.tgname,
   jsonb_build_object('definition',pg_get_triggerdef(t.oid),'enabled',t.tgenabled)
 FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid JOIN namespaces n ON n.oid=c.relnamespace
 WHERE NOT t.tgisinternal
 UNION ALL
 SELECT 'relations',n.nspname||'.'||c.relname,
   jsonb_build_object('kind',c.relkind,'owner',pg_get_userbyid(c.relowner),
                     'rls',c.relrowsecurity,'force_rls',c.relforcerowsecurity,
    'acl',(SELECT jsonb_agg(jsonb_build_array(pg_get_userbyid(a.grantor),
       CASE WHEN a.grantee=0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee) END,
       a.privilege_type,a.is_grantable) ORDER BY pg_get_userbyid(a.grantor),
       CASE WHEN a.grantee=0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee) END,a.privilege_type,a.is_grantable)
     FROM aclexplode(coalesce(c.relacl,acldefault(CASE WHEN c.relkind='S' THEN 'S'::"char" ELSE 'r'::"char" END,c.relowner))) a))
 FROM pg_class c JOIN namespaces n ON n.oid=c.relnamespace
 UNION ALL
 SELECT 'constraints',n.nspname||'.'||c.relname||'.'||x.conname,
   jsonb_build_object('definition',pg_get_constraintdef(x.oid),'type',x.contype,
     'validated',x.convalidated,'deferrable',x.condeferrable,'deferred',x.condeferred,
     'no_inherit',x.connoinherit,
     'enforced',coalesce((to_jsonb(x)->>'conenforced')::boolean,true))
 FROM pg_constraint x JOIN pg_class c ON c.oid=x.conrelid JOIN namespaces n ON n.oid=c.relnamespace
 UNION ALL
 SELECT 'indexes',schemaname||'.'||indexname,to_jsonb(indexdef)
 FROM pg_indexes WHERE schemaname IN ('public','epoch_guard')
 UNION ALL
 SELECT 'table_privileges',concat_ws('.',table_schema,table_name,grantor,grantee,privilege_type),
   to_jsonb(p)-'table_catalog'
 FROM information_schema.table_privileges p WHERE table_schema IN ('public','epoch_guard')
 UNION ALL
 SELECT 'column_privileges',concat_ws('.',table_schema,table_name,column_name,grantor,grantee,privilege_type),
   to_jsonb(p)-'table_catalog'
 FROM information_schema.column_privileges p WHERE table_schema IN ('public','epoch_guard')
 UNION ALL
 SELECT 'extensions',e.extname,jsonb_build_object('version',e.extversion,'schema',n.nspname,
   'owner',pg_get_userbyid(e.extowner),'relocatable',e.extrelocatable,
   'config',(SELECT jsonb_agg(jsonb_build_array(c.oid::regclass::text,e.extcondition[i]) ORDER BY i)
     FROM unnest(e.extconfig) WITH ORDINALITY u(oid,i) JOIN pg_class c ON c.oid=u.oid))
 FROM pg_extension e JOIN pg_namespace n ON n.oid=e.extnamespace
 UNION ALL
 SELECT 'extension_members',e.extname||':'||d.classid::regclass::text||':'||i.identity,
   jsonb_build_object('type',i.type,'schema',i.schema,'name',i.name,'identity',i.identity)
 FROM pg_depend d JOIN pg_extension e ON e.oid=d.refobjid
 CROSS JOIN LATERAL pg_identify_object(d.classid,d.objid,d.objsubid) i
 WHERE d.refclassid='pg_extension'::regclass AND d.deptype='e'
 UNION ALL
 SELECT 'database','current',jsonb_build_object('owner',pg_get_userbyid(d.datdba),
   'encoding',pg_encoding_to_char(d.encoding),'collate',d.datcollate,'ctype',d.datctype,
   'locale_provider',d.datlocprovider,'collation_version',d.datcollversion,'connection_limit',d.datconnlimit,
   'acl',(SELECT jsonb_agg(jsonb_build_array(pg_get_userbyid(a.grantor),
     CASE WHEN a.grantee=0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee) END,a.privilege_type,a.is_grantable)
     ORDER BY pg_get_userbyid(a.grantor),CASE WHEN a.grantee=0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee) END,a.privilege_type,a.is_grantable)
     FROM aclexplode(coalesce(d.datacl,acldefault('d',d.datdba))) a))
 FROM pg_database d WHERE d.datname=current_database()
)
SELECT coalesce(jsonb_object_agg(kind,items),'{}') FROM (
 SELECT kind,jsonb_object_agg(identity,value) AS items FROM objects GROUP BY kind
) grouped
"""


def compare(source, restored):
    """Return every changed object, including values; never ignore ownership or ACLs."""
    differences = []
    for kind in sorted(set(source) | set(restored)):
        left, right = source.get(kind, {}), restored.get(kind, {})
        for identity in sorted(set(left) | set(right)):
            if identity not in left or identity not in right or left[identity] != right[identity]:
                differences.append({'category': kind, 'object': identity,
                                    'source': left.get(identity), 'restored': right.get(identity)})
    return differences


def require_equal(source, restored, evidence):
    differences = compare(source, restored)
    report = {'equal': not differences, 'differences': differences,
              'source_counts': {k: len(v) for k,v in source.items()},
              'restored_counts': {k: len(v) for k,v in restored.items()}}
    (evidence / 'schema-comparison.json').write_text(json.dumps(report, indent=2)+'\n')
    if differences:
        # This runs on synthetic CI schemas, never customer records. Retain
        # bounded constraint definitions so a runner teardown cannot erase
        # the only evidence needed to distinguish drift from comparison bugs.
        print(json.dumps({'stage': 'schema_restore_mismatch', 'count': len(differences),
                          'objects': [{'category': d['category'], 'object': d['object'],
                                       **({'source': str(d['source'])[:2000],
                                           'restored': str(d['restored'])[:2000]}
                                          if d['category']=='constraints' and i<10 else {})}
                                      for i,d in enumerate(differences[:50])]}), flush=True)
        raise AssertionError('restored_schema_metadata_differs_see_schema_comparison')
    return report
