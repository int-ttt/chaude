task: make master default, delete main (local+origin)
verdict: done
ancestry: origin/main ⊂ master YES; local main ⊂ master YES; master == origin/master
open_prs_base_main: 0 (none retargeted)
gh_default_branch: master (was main)
remote_delete: git push origin --delete main -> deleted (no protection error)
local_delete: git branch -d main -> ok
origin_HEAD: repointed to origin/master (git remote set-head)
ls_remote_heads_after: fable-windows-install, interactive-child-compare, master
main_refs_remaining: none branch refs; hits are src/main/intt paths and prose only (README.md:43,76,77,90,200; .github/workflows/{e2e,release,test}.yml paths)
