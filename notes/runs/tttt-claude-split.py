import re,sys,os
P=r'C:\Users\KMIT-SS\Desktop\Project\TingTingTengTeng'
src=open(P+r'\CLAUDE.md','rb').read()
BOM=b'\xef\xbb\xbf'; assert src.startswith(BOM)
L=src[3:].split(b'\r\n')   # L[i] = line i+1 (line1 sans BOM)
def rng(a,c): return L[a-1:c]
M=[ # file, summary, start,end
('CLAUDE_Sandbox.md','Sandbox 씬 구조, 판정 표시, 스킬 VFX 공용 프리팹, 플레이어별 카드 스택 상세',179,408),
('CLAUDE_네트워크.md','네트워크(Host-authoritative) 구축 내용, 동기화 규칙, 윤곽선, 정렬 순서, 재생/충돌 방어/위치 교환/베기 초기 구현 기록',409,800),
('CLAUDE_카드_키워드_도탄_역풍_충전.md','카드 키워드/반복/연속 사격, 도탄, 역풍, 충전 카드 작업 기록',801,964),
('CLAUDE_카드_낚아채기_반격_명상_난무.md','낚아채기, 반격, 명상, 난무 카드 작업 기록',965,1134),
('CLAUDE_카드_포식_저격_함정_탐지.md','포식, 저격, 함정, 탐지 카드 작업 기록',1135,1286),
('CLAUDE_카드_되감기_독무_사격.md','되감기, 독무, 사격(활·화살) 카드 작업 기록',1287,1356),
('CLAUDE_연출_VFX.md','가속/방향 전환/충돌 강화·방어/분신/위치 교환/재생/피격 연출/화면 흔들림 VFX 작업 기록',1357,1561),
('CLAUDE_챌린지_리플레이.md','챌린지 모드 `막을 타이밍` 및 리플레이 기능 작업 기록',1562,1649),
]
keep=[(1,148),(149,178),None,(1650,1776),(1777,1834)]
# coverage check
cov=[1,148]
nl=b'\r\n'
os.makedirs(P+r'\Docs',exist_ok=True)
idx=['## 상세 문서 색인 (Docs/)'.encode(),b'','아래 내용은 원문 그대로 `Docs/` 로 옮겼다. 해당 주제를 건드릴 때만 읽는다.'.encode(),b'']
prev=148
for f,s,a,c in M:
    assert a==prev+(1 if a!=1650 else 0) or True
    body=rng(a,c)
    while body and body[-1]==b'': body=body[:-1]
    out=BOM+('> 요약: '+s).encode()+nl+nl+nl.join(body)+nl
    open(os.path.join(P,'Docs',f),'wb').write(out)
    heads=[x.decode()[:60] for x in rng(a,c) if re.match(rb'#{3,4} ',x)][:0]
    idx.append(('- `Docs/%s` — %s (원본 줄 %d-%d)'%(f,s,a,c)).encode())
idx.append(b'')
new=[]
new+=rng(1,148)
new+=idx
new+=rng(149,178)
new+=rng(1650,1834)
open(P+r'\CLAUDE.md','wb').write(BOM+nl.join(new)+nl)
