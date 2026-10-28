import json, os, subprocess, sys
env = {k: v for k, v in os.environ.items() if k != "ANTHROPIC_API_KEY"}
extra_env, flags = {}, []
if sys.argv[1] == "env": extra_env = {"CLAUDE_CODE_AUTO_COMPACT_WINDOW": "300000"}
if sys.argv[1] == "flag": flags = ["--autocompact", "300000"]
env.update(extra_env)
m = {"type":"user","message":{"role":"user","content":"/context"}}
r = subprocess.run(["claude","-p","--input-format","stream-json","--output-format","stream-json","--verbose","--model","sonnet"]+flags,
    input=json.dumps(m)+"\n", capture_output=True, text=True, env=env, timeout=120)
for l in r.stdout.splitlines():
    d = json.loads(l)
    if d["type"] == "result": print(sys.argv[1], d["result"][:1500])
