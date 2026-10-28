import pty, os, sys, time, select, re, struct, fcntl, termios
mode=sys.argv[1]; d=os.path.abspath(f"tests/e2e-hang/{mode}"); os.makedirs(d,exist_ok=True)
for f in ("done.txt",): 
    try: os.remove(d+"/"+f)
    except: pass
env=dict(os.environ); env["TERM"]="xterm-256color"
if mode=="plain": env["HARNESS_PLAIN"]="1"
pid,fd=pty.fork()
if pid==0:
    os.chdir(d); os.execve(os.environ.get("CHAUDE",os.path.expanduser("~/.local/bin/chaude")),["chaude","repl","--metrics-dir","","--transcript",d+"/t.jsonl","--cwd",d],env)
fcntl.ioctl(fd,termios.TIOCSWINSZ,struct.pack("HHHH",40,140,0,0))
log=open(d+"/out.log","wb"); t0=time.time(); buf=b""; sent=False; answered=0
prompt=("Use the Agent tool with run_in_background=true and model haiku to launch ONE subagent whose task is: "
 "run the Bash command `echo ok > done.txt` in the cwd, then reply 'done'. After launching, reply only 'launched' and end your turn immediately. Do not wait for it.")
def ev(s): print(f"[{time.time()-t0:6.1f}s] {s}",flush=True)
while time.time()-t0<200:
    r,_,_=select.select([fd],[],[],1)
    if r:
        try: c=os.read(fd,65536)
        except OSError: ev("child exited"); break
        if not c: break
        if b']11;?' in c: os.write(fd,b'\x1b]11;rgb:0000/0000/0000\x1b\\')
        if b'\x1b[6n' in c: os.write(fd,b'\x1b[1;1R')
        log.write(c); log.flush(); buf+=c
    txt=re.sub(rb"\x1b\[[0-9;?]*[a-zA-Z]",b"",buf).decode("utf8","ignore")
    if not sent and time.time()-t0>3:
        os.write(fd,prompt.encode()); time.sleep(1.5); os.write(fd,b"\r"); sent=True; ev("prompt sent")
    if mode=="tui" and ("Allow Bash" in txt[-600:] or "Allow " in txt[-300:]) and answered<5 and txt.count("Allow ")>answered:
        answered+=1; ev("permission prompt seen; answering y"); time.sleep(0.5); os.write(fd,b"y")
    if os.path.exists(d+"/done.txt") and 'ok' in open(d+"/done.txt").read():
        ev("done.txt exists"); 
        if time.time()-t0>0: end=time.time()+float(os.environ.get("POST","8"))
        while time.time()<end:
            r,_,_=select.select([fd],[],[],1)
            if r:
                try: c=os.read(fd,65536)
                except OSError: break
                log.write(c); log.flush()

        break
ev("end; done.txt=%s"%os.path.exists(d+"/done.txt"))
try:
    os.write(fd,b"\x03"); time.sleep(0.5); os.write(fd,b"/exit\r"); time.sleep(1); os.kill(pid,9)
except Exception: pass
