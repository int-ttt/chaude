# Background agent survival test (claude 2.1.312)
method: python script (scratchpad t/run.py) spawns claude -p stream-json, model haiku, bypassPermissions; sends 1 prompt: Agent(run_in_background) runs "sleep 20; date > agent_result.txt"; then sends NO more input, stdin kept open. Two runs in parallel: alive, kill (SIGKILL at t=9s). No code changed.

## alive run
- turn1 result "launched" at 5.4s (parent turn ended while agent still running)
- agent task_started 3.9s; bash task_notification 26.6s; result file created ~27s (content: Wed Oct 28 23:31:01 KST 2026)
- with no input sent, process auto-resumed a new turn on its own (assistant text 30s, final result/success at 32.9s) reporting agent done
- process still alive (poll=None) at 37s end; killed by script after
- conclusion: agent completes + writes file while process alive without input; completion is also pushed back as a new turn (parent wakes itself)

## kill run
- SIGKILL parent at 9.0s (agent mid-sleep); exit rc=-9
- result file STILL created (~28s, content 23:31:01)
- caveat: the sleep/date shell child is an orphaned OS process, survives SIGKILL of parent; so this shows the OS-level Bash survives, NOT that the subagent LLM loop continues. The subagent's own follow-up turns cannot be shown (no events after kill). Agent-level continuation after kill: unproven, probably dead.

## logs
### alive
```
[   0.0s] sent prompt; stdin stays open, no more input
[   0.7s] system/init 
[   3.8s] assistant/ ["Agent"]
[   3.9s] system/background_tasks_changed 
[   3.9s] system/task_started 
[   3.9s] user/ 
[   3.9s] rate_limit_event/ 
[   5.4s] assistant/ ["launched"]
[   5.4s] rate_limit_event/ 
[   5.4s] result/success launched
[   5.9s] assistant/ ["Bash"]
[   5.9s] system/task_progress 
[   9.6s] system/task_started 
[  26.6s] system/task_notification 
[  26.6s] user/ 
[  27.1s] RESULT FILE CREATED: Wed Oct 28 23:31:01 KST 2026
[  29.7s] assistant/ [""]
[  30.1s] assistant/ ["The command ran. It slept 20 seconds and then wrote the current date to `/privat"]
[  30.1s] rate_limit_event/ 
[  30.2s] system/task_updated 
[  30.2s] system/task_notification 
[  30.2s] system/background_tasks_changed 
[  30.2s] system/init 
[  32.0s] system/thinking_tokens 
[  32.6s] system/thinking_tokens 
[  32.6s] assistant/ [""]
[  32.9s] assistant/ ["\ubc31\uadf8\ub77c\uc6b4\ub4dc \uc5d0\uc774\uc804\ud2b8\uac00 \uc644\ub8cc\ub418\uc5c8\uc2b5\ub2c8\ub2e4. 20\ucd08 \u
[  32.9s] result/success 백그라운드 에이전트가 완료되었습니다. 20초 대기 후 `agent_result.txt`에 날짜를 기록했다고 보고했습니다. 파일 내용은 확인하지 않았습니다.
[  37.1s] end: file_exists=True poll=None
```
### kill
```
[   0.0s] sent prompt; stdin stays open, no more input
[   0.6s] system/init 
[   2.3s] system/thinking_tokens 
[   3.4s] system/thinking_tokens 
[   3.7s] system/thinking_tokens 
[   3.7s] assistant/ [""]
[   4.5s] assistant/ ["Agent"]
[   4.5s] system/background_tasks_changed 
[   4.5s] system/task_started 
[   4.5s] user/ 
[   4.5s] rate_limit_event/ 
[   6.0s] assistant/ ["launched"]
[   6.0s] rate_limit_event/ 
[   6.0s] result/success launched
[   6.6s] assistant/ ["Bash"]
[   6.6s] system/task_progress 
[   9.0s] KILLED process
[  10.0s] process exited rc=-9
[  28.1s] RESULT FILE CREATED: Wed Oct 28 23:31:01 KST 2026
[  37.1s] end: file_exists=True poll=-9
```
