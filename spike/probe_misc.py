"""Spike: hook events (cwd=claude-lean), slash cmd via -p, image block, --session-id."""
import json, os, subprocess, base64, uuid, collections
ENV = {k: v for k, v in os.environ.items() if k != "ANTHROPIC_API_KEY"}
PNG = base64.b64encode(bytes.fromhex("89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4890000000d4944415478da63f8cfc0f01f0005000201a5f645400000000049454e44ae426082")).decode()
sid = str(uuid.uuid4())
msgs = [
 {"type":"user","message":{"role":"user","content":[{"type":"image","source":{"type":"base64","media_type":"image/png","data":PNG}},{"type":"text","text":"What is this image's size in pixels? One short sentence, no tools."}]}},
 {"type":"user","message":{"role":"user","content":"/context"}},
]
r = subprocess.run(["claude","-p","--input-format","stream-json","--output-format","stream-json","--verbose","--include-hook-events","--model","haiku","--session-id",sid,"--permission-prompts","none"],
    input="".join(json.dumps(m)+"\n" for m in msgs), capture_output=True, text=True, env=ENV,
    cwd="/Users/leekwangseok/PycharmProjects/claude-lean", timeout=180)
open("samples/probe_misc.jsonl","w").write(r.stdout)
L=[json.loads(l) for l in r.stdout.splitlines()]
print("rc",r.returncode,r.stderr[:200]); print("sid used", sid, {d.get('session_id') for d in L}=={sid})
print(collections.Counter((d['type'],d.get('subtype')) for d in L if d['type']!='stream_event'))
for d in L:
    if d['type']=='result': print("RESULT", d['subtype'], str(d.get('result'))[:300])
