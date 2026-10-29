import glob,re,os,collections
P=r'C:\Users\KMIT-SS\Desktop\Project\TingTingTengTeng'
import subprocess
orig=open(os.path.join(os.environ['TEMP'],'CLAUDE.orig'),'rb').read()
ol=orig[3:].split(b'\r\n')
files=[os.path.join(P,'CLAUDE.md')]+sorted(glob.glob(os.path.join(P,'Docs','CLAUDE_*.md')))
cnt=collections.Counter();allb=b''
for f in files:
    b=open(f,'rb').read()
    assert b.startswith(b'\xef\xbb\xbf'),f
    assert b.count(b'\n')==b.count(b'\r\n'),f
    for x in b[3:].split(b'\r\n'):
        if re.match(rb'#{2,4} ',x): cnt[x]+=1
# fence-safe: orig heading counts
oc=collections.Counter(x for x in ol if re.match(rb'#{2,4} ',x))
bad=[(h.decode()[:50],oc[h],cnt[h]) for h in oc if cnt[h]!=oc[h]]
print('orig headings',sum(oc.values()),'distinct',len(oc),'new total',sum(cnt.values()),'mismatch',bad)
print('dup in orig',[h.decode()[:40] for h,n in oc.items() if n>1])
# line multiset
nl=collections.Counter()
for f in files:
    for x in open(f,'rb').read()[3:].split(b'\r\n'): nl[x]+=1
oo=collections.Counter(ol); miss=oo-nl
print('orig lines missing in new:',sum(miss.values()))
