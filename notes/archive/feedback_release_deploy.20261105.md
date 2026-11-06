---
name: release-deploy-readme
description: next releases deploy to this Mac and update README.md in the same pass
metadata:
  node_type: memory
  type: feedback
  originSessionId: c2b3c957-ce0e-4d7c-b143-86493f3022ef
  modified: 2026-11-01T01:51:57.440Z
---

다음 릴리즈부터 이 맥에 배포하고 README.md를 같은 작업에서 함께 수정한다.

Why: user asked on 2026-11-01 after v0.9.1 release.
How to apply: at each release, include README.md update and local deploy in the same run; deploy target still unconfirmed, ask user before pushing deploy target.
