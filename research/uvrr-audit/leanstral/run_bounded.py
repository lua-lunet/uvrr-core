import os, pathlib, subprocess, time, signal, json
root=pathlib.Path('/Users/Shared/lua-lunet/vrr-core')
work=root/'.tmp/uvrr-audit/leanstral'
env=os.environ.copy()
secrets=[]
for line in (root/'.env').read_text().splitlines():
    if line.strip() and not line.lstrip().startswith('#') and '=' in line:
        key,val=line.split('=',1)
        key=key.strip().removeprefix('export ')
        val=val.strip().strip('\"\'')
        if key=='MISTRAL_API_KEY': env[key]=val; secrets.append(val)
argv=['/Users/consensussolutions/opt/mistral-vibe-fork/bin/vibe','-p',(work/'PROMPT-weighted-general.txt').read_text(),'--agent','lean','--auto-approve','--trust','--max-turns','12','--max-price','2.0','--max-tokens','28000','--output','streaming','--workdir',str(work)]
started=time.monotonic(); reason='completed'; maxrss=0
raw=work/'raw-private.log'
with raw.open('w') as out:
    os.chmod(raw,0o600)
    p=subprocess.Popen(argv,cwd=work,env=env,stdout=out,stderr=subprocess.STDOUT,start_new_session=True)
    while p.poll() is None:
        rows=subprocess.check_output(['ps','-axo','pid=,ppid=,rss='],text=True).splitlines()
        procs=[tuple(map(int,r.split())) for r in rows if len(r.split())==3]
        ids={p.pid}
        for _ in range(12): ids.update(pid for pid,ppid,rss in procs if ppid in ids)
        rss=sum(rss for pid,ppid,rss in procs if pid in ids); maxrss=max(maxrss,rss)
        if time.monotonic()-started>180 or rss>2*1024*1024:
            reason='wall_timeout' if time.monotonic()-started>180 else 'memory_limit'
            for pid in reversed(sorted(ids)):
                try: os.kill(pid,signal.SIGKILL)
                except ProcessLookupError: pass
            break
        time.sleep(1)
    rc=p.wait()
log=raw.read_text()
for secret in secrets:
    if secret: log=log.replace(secret,'[REDACTED]')
(work/'sanitized-run.jsonl').write_text(log)
report={'agent':'lean','model_from_builtin_profile':'labs-leanstral-1-5','max_turns':12,'max_price_usd':2.0,'max_total_tokens':28000,'wall_limit_seconds':180,'tree_rss_limit_kib':2*1024*1024,'peak_tree_rss_kib':maxrss,'termination':reason,'returncode':rc,'elapsed_seconds':round(time.monotonic()-started,2)}
(work/'run-summary.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report))
