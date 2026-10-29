version: 0.7.1 (prev v0.7.0)
vet+test: PASS
review: sonnet-reviewer FIX (notes/runs/release-review.md); fixed scout effort->high, switchSession keeps pendingSnapshot unless resume set; bumped cmd/chaude/main.go
not fixed (low/med, deferred): haiku->haiku worker switch not parked; usageFetch no timeout; pickTarget unused kind param
commit: 18f85d2 + b6e057f (metrics), master merged into main, pushed main/master/v0.7.1
release workflow: success; notes have "Changes since v0.7.0"
install: curl install.sh 404 (private repo); sh install.sh (gh path) -> chaude 0.7.1 PASS
