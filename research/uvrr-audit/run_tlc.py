import hashlib, json, pathlib, subprocess, sys, time
root=pathlib.Path('/Users/Shared/lua-lunet/vrr-core')
out=root/'.tmp/uvrr-audit/tlc'
cfg=sys.argv[1]
model='VrrCoreEras.tla'
start=time.monotonic()
argv=['java','-Xmx2g','-XX:+UseParallelGC','-jar',str(root/'.tmp/tla-hello/tla2tools.jar'),
      '-workers','2','-metadir',str(out/(cfg+'.states')),'-config',cfg,model]
with (out/(cfg+'.log')).open('w') as log:
 try:
  result=subprocess.run(argv,cwd=root/'formal',stdout=log,stderr=subprocess.STDOUT,timeout=180)
  status='exited';code=result.returncode
 except subprocess.TimeoutExpired:
  status='timeout';code=None
data={'config':cfg,'model':model,'status':status,'exit_code':code,
      'elapsed_seconds':round(time.monotonic()-start,2),'wall_limit_seconds':180,
      'java_heap_limit':'2g','workers':2,
      'sha256':{f:hashlib.sha256((root/'formal'/f).read_bytes()).hexdigest() for f in [cfg,model]}}
(out/(cfg+'.json')).write_text(json.dumps(data,indent=2)+'\n')
print(json.dumps(data))
for line in (out/(cfg+'.log')).read_text().splitlines():
 if any(m in line for m in ['Invariant ','Model checking completed','distinct states found','Error:']):
  print(line)
