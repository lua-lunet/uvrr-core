import json, os, pathlib, re, time, urllib.request
root = pathlib.Path('/Users/Shared/lua-lunet/vrr-core')
work = pathlib.Path(__file__).parent
key = os.environ.get('MISTRAL_API_KEY')
if not key:
    for line in (root / '.env').read_text().splitlines():
        match = re.match(r'\s*(?:export\s+)?MISTRAL_API_KEY\s*=\s*(.*)', line)
        if match:
            key = match[1].strip().strip('\"\'')
assert key, 'Missing credential'
prompt = ('Return only a Lean tactic proof replacing the single sorry below. '
          'Lean 4.33.1 core only; no imports or changes to definitions or statement. '
          'Use induction on nodes, specialize disjointness at the head and tail, '
          'split the two Boolean membership tests, unfold total/distance and use omega. '
          'Do not use Mathlib identifiers.\n' + (work/'input.lean').read_text())
(work/'prompt.txt').write_text(prompt)
payload = {'model': 'labs-leanstral-2603', 'temperature': 0, 'max_tokens': 1800,
           'messages': [{'role':'user','content':prompt}]}
req = urllib.request.Request('https://api.mistral.ai/v1/chat/completions',
    data=json.dumps(payload).encode(), headers={'Content-Type':'application/json',
    'Authorization':'Bearer '+key})
started = time.monotonic()
try:
    with urllib.request.urlopen(req, timeout=60) as response:
        result=json.load(response)
except Exception as exc:
    print(type(exc).__name__, getattr(exc, 'code', ''))
    raise SystemExit(1)
text=result['choices'][0]['message']['content']
if key in text: text=text.replace(key,'[REDACTED]')
(work/'response.txt').write_text(text)
report={'model_requested':payload['model'],'model_returned':result.get('model'),
        'usage':result.get('usage'),'max_output_tokens':1800,
        'elapsed_seconds':round(time.monotonic()-started,2),
        'finish_reason':result['choices'][0].get('finish_reason'),
        'billed_cost':'not reported'}
(work/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report))
