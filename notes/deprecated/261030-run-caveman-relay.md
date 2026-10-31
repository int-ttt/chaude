# run: caveman relay (1회, 축약 실행)
work_dir: /tmp/caveman-test-1793366508
question_file: /tmp/caveman-test-1793366508/question.txt
cap_usd: 1.00
done: A (opus plain) total_cost_usd=0.5437, input=26, cache_create=43241, cache_read=368727, output=6196 (thinking=1413, visible=4783)
stopped_before: B opus call
reason: cap rule. est_cum = 0.5437 (A actual) + 0.5437 (B est, same scale) = 1.087 > 1.00
not_run: B (opus caveman), B translate (haiku), C, A2, checker, reviewer
notes: billing.md has no model unit prices -> cost from CLI total_cost_usd only
