# Usage aggregate: Claude Code transcripts

source: /Users/leekwangseok/.claude/projects/**/*.jsonl (read-only, aggregates only, no content)
generated: 2026-10-29 (script /tmp/usage_agg/agg.py, python3 -I)
time basis: timestamps converted to UTC; day = UTC date; hour = UTC hour

## Totals
- jsonl files found: 534 (count of .jsonl files under projects dirs)
- project dirs with files: 92 (distinct subdirectories containing .jsonl)
- distinct sessionIds: 175 (unique sessionId values across all entries)
- lines: 49276 (non-empty JSONL lines)
- unparseable lines: 0 (JSON decode failures, skipped)
- lines without parseable timestamp: 11384 (excluded from day/hour counts)
- timestamp range (UTC): 2026-09-08T01:20:14.590000+00:00 to 2026-10-29T06:27:15.963000+00:00

## Per project dir (files = .jsonl count; sessions = distinct sessionId; lines = non-empty lines)
| project dir | files | sessions | lines |
|---|---|---|---|
| -Users-leekwangseok-PycharmProjects-claude-harness | 117 | 117 | 8598 |
| -Users-leekwangseok-Documents-univ/3e384a95-59c3-4f67-acbd-16ae398a1831/subagents | 62 | 1 | 5012 |
| -Users-leekwangseok-PycharmProjects-claude-harness/19fde733-87b5-4f1b-952b-c584071b9a22/subagents | 43 | 1 | 4707 |
| -Users-leekwangseok-PycharmProjects-claude-lean/0ca417e2-69c8-4213-ac04-8667b074991b/subagents | 40 | 1 | 2262 |
| -Users-leekwangseok-WebstormProjects-TextGame/b0fcc67b-8ba4-4322-82c9-22b1914c747e/subagents | 28 | 1 | 3098 |
| -Users-leekwangseok-PycharmProjects-claude-harness/17d8f798-c6d3-4eae-a74a-b887226f4917/subagents | 26 | 1 | 1837 |
| -Users-leekwangseok-PycharmProjects-claude-harness/d426c75b-4839-4141-bd45-d1788c799901/subagents | 25 | 1 | 2032 |
| -Users-leekwangseok-PycharmProjects-claude-harness/a14a3dd8-0799-4888-9ac8-e20ae1f03237/subagents | 19 | 1 | 1247 |
| -Users-leekwangseok-PycharmProjects-claude-harness/83fc90cf-23b4-4033-ac95-28d2c709ca60/subagents | 15 | 1 | 767 |
| -Users-leekwangseok-PycharmProjects-claude-harness/26e84f4e-31c8-40df-b7ab-9f94fa83636c/subagents | 12 | 1 | 871 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-tui | 8 | 8 | 369 |
| -Users-leekwangseok-WebstormProjects-TextGame | 8 | 8 | 1401 |
| -Users-leekwangseok-PycharmProjects-claude-harness/967c745d-d3d0-4118-8d80-aac662ff55d7/subagents | 6 | 1 | 313 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-rebuild | 5 | 5 | 263 |
| -Users-leekwangseok-WebstormProjects-TextGame/2532c82b-3774-4e55-a6e9-bb9b00d5f2c5/subagents | 5 | 1 | 177 |
| -Users-leekwangseok-PycharmProjects-claude-harness-internal-harness | 4 | 4 | 121 |
| -Users-leekwangseok-PycharmProjects-claude-harness/393c1ac7-e19f-4d6f-9e67-ef832f5399e5/subagents | 4 | 1 | 285 |
| -Users-leekwangseok-PycharmProjects-claude-harness/cc5438a2-143e-422c-855f-303022a13a5d/subagents | 4 | 1 | 297 |
| -Users-leekwangseok-PycharmProjects-claude-lean | 3 | 3 | 1320 |
| -private-tmp-claude-501-sc-e2e2 | 3 | 3 | 150 |
| -Users-leekwangseok-PycharmProjects-claude-harness/98629f39-4e0f-4b41-8176-9e9654bed84f/subagents | 3 | 1 | 325 |
| -Users-leekwangseok-PycharmProjects-claude-harness/d4657cd7-e546-4a34-8197-b0af0de4b40d/subagents | 3 | 1 | 415 |
| -Users-leekwangseok-PycharmProjects-claude-harness/198056b4-24f4-4ed8-8d6b-3fa9ffe1c9fe/subagents | 3 | 1 | 183 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad-w2 | 3 | 3 | 158 |
| -Users-leekwangseok-PycharmProjects-VocalAnalyze | 3 | 3 | 8011 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad-w | 3 | 3 | 158 |
| -private-tmp-claude-501-sc-e2e | 3 | 3 | 149 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-rebuild/a7cc1758-ca68-4dab-9487-5197863665d4/subagents | 2 | 1 | 70 |
| -Users-leekwangseok-PycharmProjects-claude-harness/2934d8ea-491f-4d4b-8507-4b536415fabb/subagents | 2 | 1 | 168 |
| -Users-leekwangseok-PycharmProjects-claude-harness/23280028-e3f0-4bbd-8146-bb40ad03e036/subagents | 2 | 1 | 74 |
| -Users-leekwangseok-PycharmProjects-claude-harness/53da31b7-fb6d-4976-a3f3-0c3dfb988d48/subagents | 2 | 1 | 185 |
| -Users-leekwangseok-PycharmProjects-claude-harness/821a856f-dbc1-4f42-9b29-79235f8e95e3/subagents | 2 | 1 | 182 |
| -Users-leekwangseok-PycharmProjects-claude-harness/69494b62-ab4b-42de-9a9b-a65b87bcac62/subagents | 2 | 1 | 96 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-plain | 2 | 2 | 92 |
| -Users-leekwangseok | 2 | 2 | 195 |
| -Users-leekwangseok-Desktop-------2-PycharmProjects-UnitV2M12 | 2 | 2 | 547 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad | 2 | 2 | 26 |
| -Users-leekwangseok-Documents-univ | 2 | 2 | 1290 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-rebuild/a313c640-2bff-40f1-aa3d-22602c716ca5/subagents | 1 | 1 | 29 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-rebuild/cdfdc245-a78f-4591-9928-42ea7c9b6bac/subagents | 1 | 1 | 63 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-rebuild/56c92a56-d29e-4d90-a4fd-16774cf25cbe/subagents | 1 | 1 | 35 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-tui/b493de1a-3511-4eb1-8eb8-ba2d4be597fd/subagents | 1 | 1 | 23 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-tui/395dafb5-4819-40be-a539-420633f4e9fc/subagents | 1 | 1 | 21 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-tui/f8b10b78-9478-4227-85ea-702feb7007c7/subagents | 1 | 1 | 22 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-tui/df72f26b-908c-4db3-adf5-b67a519a04d2/subagents | 1 | 1 | 22 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-tui/5d7ccf7c-87c2-4ba2-bcd9-247e0eb5391f/subagents | 1 | 1 | 21 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-tui/3b89f0b6-4d07-4454-a56d-3571f0edb959/subagents | 1 | 1 | 21 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-tui/fbe09ff3-ed9d-4534-947a-b4fca1e4e26f/subagents | 1 | 1 | 22 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-tui/f36f8b0e-54d9-44a5-9a31-21a232d22022/subagents | 1 | 1 | 21 |
| -private-tmp | 1 | 1 | 51 |
| -private-tmp-claude-501-sc-e2e2/23a6c8ab-dcd4-4cff-a6a7-9092cb5ccf51/subagents | 1 | 1 | 41 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad-t-kill | 1 | 1 | 36 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad-t-kill/bc42ef04-912b-4d8a-9617-8c6c955a2568/subagents | 1 | 1 | 15 |
| -Users-leekwangseok-PycharmProjects-claude-harness/8b944b19-9c32-4391-b6da-857f09943a12/subagents | 1 | 1 | 22 |
| -Users-leekwangseok-PycharmProjects-claude-harness/d5e8b2e6-2271-4fef-92cc-7eb01f86f29d/subagents | 1 | 1 | 21 |
| -Users-leekwangseok-PycharmProjects-claude-harness/a708eafa-ea38-4891-aabb-a6173290d6dc/subagents | 1 | 1 | 21 |
| -Users-leekwangseok-PycharmProjects-claude-harness/f70be273-b2da-4b16-b488-d5d8677822e0/subagents | 1 | 1 | 60 |
| -Users-leekwangseok-PycharmProjects-claude-harness/7658b91f-b814-4619-b6ac-784ddfed7673/subagents | 1 | 1 | 32 |
| -Users-leekwangseok-PycharmProjects-claude-harness/be03c054-1d63-4c11-96b9-25f6962c46b4/subagents | 1 | 1 | 71 |
| -Users-leekwangseok-PycharmProjects-claude-harness/d66e7ec9-f680-4dd7-a41f-7856d555a42b/subagents | 1 | 1 | 49 |
| -Users-leekwangseok-PycharmProjects-claude-harness/3fb2e9b9-5343-4353-b180-6c1e83fd28c3/subagents | 1 | 1 | 45 |
| -Users-leekwangseok-PycharmProjects-claude-harness/87e33a13-73d4-4c01-8611-5b09f27ad36f/subagents | 1 | 1 | 21 |
| -Users-leekwangseok-PycharmProjects-claude-harness/4e08cd52-8060-433e-bbf8-de64dfd6b91e/subagents | 1 | 1 | 22 |
| -Users-leekwangseok-PycharmProjects-claude-harness/27384857-efb6-470b-9a20-f90f2d487f29/subagents | 1 | 1 | 50 |
| -Users-leekwangseok-PycharmProjects-claude-harness/24f37662-5f04-4080-96ee-e3634e0a43e8/subagents | 1 | 1 | 22 |
| -Users-leekwangseok-PycharmProjects-claude-harness/c7309dba-f290-489f-ada0-c03a2cb44d59/subagents | 1 | 1 | 48 |
| -Users-leekwangseok-PycharmProjects-claude-harness/8fb4ca9f-5474-41af-bb9b-6378e71a26b3/subagents | 1 | 1 | 22 |
| -Users-leekwangseok-PycharmProjects-claude-harness/bbc79bda-4589-4566-80be-4fff12bd2e64/subagents | 1 | 1 | 21 |
| -Users-leekwangseok-PycharmProjects-claude-harness/924ad765-4b56-4ebe-a9de-1d0af816cd7b/subagents | 1 | 1 | 23 |
| -Users-leekwangseok-PycharmProjects-claude-harness/9580ae8b-8cf3-4978-bef7-f7d2ced01b62/subagents | 1 | 1 | 21 |
| -Users-leekwangseok-PycharmProjects-claude-harness/031c909d-9e1e-444b-8bb8-42f9bebcbe81/subagents | 1 | 1 | 76 |
| -Users-leekwangseok-PycharmProjects-claude-harness/737b5d1e-096c-4d5a-90b0-386d39236b21/subagents | 1 | 1 | 20 |
| -Users-leekwangseok-PycharmProjects-claude-harness/049010ed-e7ae-4892-82e9-470d2a6998a9/subagents | 1 | 1 | 49 |
| -Users-leekwangseok-PycharmProjects-claude-harness/3901c17e-f331-43ad-9936-ab5f9511fa05/subagents | 1 | 1 | 21 |
| -Users-leekwangseok-PycharmProjects-claude-harness/39c32ee4-a24b-46ae-8546-ae3604409f44/subagents | 1 | 1 | 22 |
| -Users-leekwangseok-PycharmProjects-claude-harness/d7f3fa62-f3d7-48bd-8626-afea1eb13118/subagents | 1 | 1 | 49 |
| -Users-leekwangseok-PycharmProjects-claude-harness/e29a8002-9db4-4a31-af4f-4f8e7c59ec95/subagents | 1 | 1 | 21 |
| -Users-leekwangseok-PycharmProjects-claude-harness/bdc51d3c-773b-4a0c-bc86-ef049da62c53/subagents | 1 | 1 | 27 |
| -Users-leekwangseok-PycharmProjects-claude-harness/0a2b7d57-1669-4f7a-8d51-6102325efb58/subagents | 1 | 1 | 23 |
| -Users-leekwangseok-PycharmProjects-claude-harness/f8f14a56-b9c8-486e-ab63-bb4e208b19ac/subagents | 1 | 1 | 22 |
| -Users-leekwangseok-PycharmProjects-claude-harness/1d28aba6-f076-4dde-b039-ae57788de95f/subagents | 1 | 1 | 45 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad-w2/9ddae8ac-043c-419e-bab9-54da69d37355/subagents | 1 | 1 | 25 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad-w2/3dfe70fe-8a8a-4639-9788-8fa75d36172e/subagents | 1 | 1 | 37 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad-w2/c45bf81a-e607-4c8d-86a8-9c4832a2ba26/subagents | 1 | 1 | 31 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-plain/722ca24f-7a49-46de-8fa5-397b0b8240ff/subagents | 1 | 1 | 23 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-plain/3249100b-9741-44db-bff2-a75a84dfd17b/subagents | 1 | 1 | 23 |
| -Users-leekwangseok-claude-opus-lean | 1 | 1 | 63 |
| -Users-leekwangseok-claude-opus-lean/9e9f8a36-9a9c-4bd8-8443-3dc8a5e0c66d/subagents | 1 | 1 | 97 |
| -private-tmp-claude-501-sc-e2e/6f30cf06-7014-4e78-96a7-ecb86929a442/subagents | 1 | 1 | 42 |
| -private-tmp-claude-501-sc | 1 | 1 | 27 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad-t-alive | 1 | 1 | 43 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad-t-alive/f4622c84-d9b9-4e98-a251-0a38a0678e9a/subagents | 1 | 1 | 25 |

## Per day (UTC; lines = entries with that date; sessions = distinct sessionIds with entries that day)
| day | lines | distinct sessions |
|---|---|---|
| 2026-09-08 | 1349 | 1 |
| 2026-09-09 | 985 | 1 |
| 2026-09-10 | 461 | 1 |
| 2026-09-11 | 380 | 1 |
| 2026-09-12 | 128 | 2 |
| 2026-09-13 | 173 | 2 |
| 2026-09-16 | 66 | 1 |
| 2026-09-21 | 12 | 1 |
| 2026-09-24 | 617 | 1 |
| 2026-09-28 | 273 | 1 |
| 2026-09-30 | 5 | 2 |
| 2026-10-02 | 1225 | 1 |
| 2026-10-03 | 354 | 1 |
| 2026-10-06 | 157 | 1 |
| 2026-10-07 | 211 | 2 |
| 2026-10-09 | 22 | 1 |
| 2026-10-10 | 172 | 1 |
| 2026-10-24 | 36 | 1 |
| 2026-10-27 | 2357 | 2 |
| 2026-10-28 | 22084 | 152 |
| 2026-10-29 | 6825 | 18 |

## Sessions per project per day (distinct sessionIds with entries on that UTC day)
| project dir | day | sessions |
|---|---|---|
| -Users-leekwangseok | 2026-10-24 | 1 |
| -Users-leekwangseok | 2026-10-28 | 1 |
| -Users-leekwangseok-Desktop-------2-PycharmProjects-UnitV2M12 | 2026-09-12 | 2 |
| -Users-leekwangseok-Desktop-------2-PycharmProjects-UnitV2M12 | 2026-09-13 | 1 |
| -Users-leekwangseok-Desktop-------2-PycharmProjects-UnitV2M12 | 2026-10-10 | 1 |
| -Users-leekwangseok-Documents-univ | 2026-10-28 | 2 |
| -Users-leekwangseok-Documents-univ/3e384a95-59c3-4f67-acbd-16ae398a1831/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-VocalAnalyze | 2026-09-08 | 1 |
| -Users-leekwangseok-PycharmProjects-VocalAnalyze | 2026-09-09 | 1 |
| -Users-leekwangseok-PycharmProjects-VocalAnalyze | 2026-09-10 | 1 |
| -Users-leekwangseok-PycharmProjects-VocalAnalyze | 2026-09-11 | 1 |
| -Users-leekwangseok-PycharmProjects-VocalAnalyze | 2026-09-13 | 1 |
| -Users-leekwangseok-PycharmProjects-VocalAnalyze | 2026-09-16 | 1 |
| -Users-leekwangseok-PycharmProjects-VocalAnalyze | 2026-09-21 | 1 |
| -Users-leekwangseok-PycharmProjects-VocalAnalyze | 2026-09-24 | 1 |
| -Users-leekwangseok-PycharmProjects-VocalAnalyze | 2026-09-28 | 1 |
| -Users-leekwangseok-PycharmProjects-VocalAnalyze | 2026-09-30 | 2 |
| -Users-leekwangseok-PycharmProjects-VocalAnalyze | 2026-10-02 | 1 |
| -Users-leekwangseok-PycharmProjects-VocalAnalyze | 2026-10-03 | 1 |
| -Users-leekwangseok-PycharmProjects-VocalAnalyze | 2026-10-06 | 1 |
| -Users-leekwangseok-PycharmProjects-VocalAnalyze | 2026-10-07 | 2 |
| -Users-leekwangseok-PycharmProjects-VocalAnalyze | 2026-10-09 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness | 2026-10-28 | 101 |
| -Users-leekwangseok-PycharmProjects-claude-harness | 2026-10-29 | 18 |
| -Users-leekwangseok-PycharmProjects-claude-harness-internal-harness | 2026-10-28 | 4 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-plain | 2026-10-28 | 2 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-plain/3249100b-9741-44db-bff2-a75a84dfd17b/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-plain/722ca24f-7a49-46de-8fa5-397b0b8240ff/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-tui | 2026-10-28 | 8 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-tui/395dafb5-4819-40be-a539-420633f4e9fc/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-tui/3b89f0b6-4d07-4454-a56d-3571f0edb959/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-tui/5d7ccf7c-87c2-4ba2-bcd9-247e0eb5391f/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-tui/b493de1a-3511-4eb1-8eb8-ba2d4be597fd/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-tui/df72f26b-908c-4db3-adf5-b67a519a04d2/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-tui/f36f8b0e-54d9-44a5-9a31-21a232d22022/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-tui/f8b10b78-9478-4227-85ea-702feb7007c7/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-hang-tui/fbe09ff3-ed9d-4534-947a-b4fca1e4e26f/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-rebuild | 2026-10-28 | 5 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-rebuild/56c92a56-d29e-4d90-a4fd-16774cf25cbe/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-rebuild/a313c640-2bff-40f1-aa3d-22602c716ca5/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-rebuild/a7cc1758-ca68-4dab-9487-5197863665d4/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness-tests-e2e-rebuild/cdfdc245-a78f-4591-9928-42ea7c9b6bac/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/031c909d-9e1e-444b-8bb8-42f9bebcbe81/subagents | 2026-10-29 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/049010ed-e7ae-4892-82e9-470d2a6998a9/subagents | 2026-10-29 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/0a2b7d57-1669-4f7a-8d51-6102325efb58/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/17d8f798-c6d3-4eae-a74a-b887226f4917/subagents | 2026-10-29 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/198056b4-24f4-4ed8-8d6b-3fa9ffe1c9fe/subagents | 2026-10-29 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/19fde733-87b5-4f1b-952b-c584071b9a22/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/1d28aba6-f076-4dde-b039-ae57788de95f/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/23280028-e3f0-4bbd-8146-bb40ad03e036/subagents | 2026-10-29 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/24f37662-5f04-4080-96ee-e3634e0a43e8/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/26e84f4e-31c8-40df-b7ab-9f94fa83636c/subagents | 2026-10-29 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/27384857-efb6-470b-9a20-f90f2d487f29/subagents | 2026-10-29 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/2934d8ea-491f-4d4b-8507-4b536415fabb/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/3901c17e-f331-43ad-9936-ab5f9511fa05/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/393c1ac7-e19f-4d6f-9e67-ef832f5399e5/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/39c32ee4-a24b-46ae-8546-ae3604409f44/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/3fb2e9b9-5343-4353-b180-6c1e83fd28c3/subagents | 2026-10-29 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/4e08cd52-8060-433e-bbf8-de64dfd6b91e/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/53da31b7-fb6d-4976-a3f3-0c3dfb988d48/subagents | 2026-10-29 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/69494b62-ab4b-42de-9a9b-a65b87bcac62/subagents | 2026-10-29 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/737b5d1e-096c-4d5a-90b0-386d39236b21/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/7658b91f-b814-4619-b6ac-784ddfed7673/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/821a856f-dbc1-4f42-9b29-79235f8e95e3/subagents | 2026-10-29 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/83fc90cf-23b4-4033-ac95-28d2c709ca60/subagents | 2026-10-29 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/87e33a13-73d4-4c01-8611-5b09f27ad36f/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/8b944b19-9c32-4391-b6da-857f09943a12/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/8fb4ca9f-5474-41af-bb9b-6378e71a26b3/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/924ad765-4b56-4ebe-a9de-1d0af816cd7b/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/9580ae8b-8cf3-4978-bef7-f7d2ced01b62/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/967c745d-d3d0-4118-8d80-aac662ff55d7/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/98629f39-4e0f-4b41-8176-9e9654bed84f/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/a14a3dd8-0799-4888-9ac8-e20ae1f03237/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/a708eafa-ea38-4891-aabb-a6173290d6dc/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/bbc79bda-4589-4566-80be-4fff12bd2e64/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/bdc51d3c-773b-4a0c-bc86-ef049da62c53/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/be03c054-1d63-4c11-96b9-25f6962c46b4/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/c7309dba-f290-489f-ada0-c03a2cb44d59/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/cc5438a2-143e-422c-855f-303022a13a5d/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/d426c75b-4839-4141-bd45-d1788c799901/subagents | 2026-10-29 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/d4657cd7-e546-4a34-8197-b0af0de4b40d/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/d4657cd7-e546-4a34-8197-b0af0de4b40d/subagents | 2026-10-29 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/d5e8b2e6-2271-4fef-92cc-7eb01f86f29d/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/d66e7ec9-f680-4dd7-a41f-7856d555a42b/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/d7f3fa62-f3d7-48bd-8626-afea1eb13118/subagents | 2026-10-29 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/e29a8002-9db4-4a31-af4f-4f8e7c59ec95/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/f70be273-b2da-4b16-b488-d5d8677822e0/subagents | 2026-10-29 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-harness/f8f14a56-b9c8-486e-ab63-bb4e208b19ac/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-PycharmProjects-claude-lean | 2026-10-28 | 3 |
| -Users-leekwangseok-PycharmProjects-claude-lean/0ca417e2-69c8-4213-ac04-8667b074991b/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-WebstormProjects-TextGame | 2026-10-27 | 2 |
| -Users-leekwangseok-WebstormProjects-TextGame | 2026-10-28 | 7 |
| -Users-leekwangseok-WebstormProjects-TextGame/2532c82b-3774-4e55-a6e9-bb9b00d5f2c5/subagents | 2026-10-27 | 1 |
| -Users-leekwangseok-WebstormProjects-TextGame/b0fcc67b-8ba4-4322-82c9-22b1914c747e/subagents | 2026-10-27 | 1 |
| -Users-leekwangseok-WebstormProjects-TextGame/b0fcc67b-8ba4-4322-82c9-22b1914c747e/subagents | 2026-10-28 | 1 |
| -Users-leekwangseok-claude-opus-lean | 2026-10-28 | 1 |
| -Users-leekwangseok-claude-opus-lean/9e9f8a36-9a9c-4bd8-8443-3dc8a5e0c66d/subagents | 2026-10-28 | 1 |
| -private-tmp | 2026-10-28 | 1 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad | 2026-10-28 | 2 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad-t-alive | 2026-10-28 | 1 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad-t-alive/f4622c84-d9b9-4e98-a251-0a38a0678e9a/subagents | 2026-10-28 | 1 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad-t-kill | 2026-10-28 | 1 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad-t-kill/bc42ef04-912b-4d8a-9617-8c6c955a2568/subagents | 2026-10-28 | 1 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad-w | 2026-10-28 | 3 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad-w2 | 2026-10-28 | 3 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad-w2/3dfe70fe-8a8a-4639-9788-8fa75d36172e/subagents | 2026-10-28 | 1 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad-w2/9ddae8ac-043c-419e-bab9-54da69d37355/subagents | 2026-10-28 | 1 |
| -private-tmp-claude-501--Users-leekwangseok-PycharmProjects-claude-harness-19fde733-87b5-4f1b-952b-c584071b9a22-scratchpad-w2/c45bf81a-e607-4c8d-86a8-9c4832a2ba26/subagents | 2026-10-28 | 1 |
| -private-tmp-claude-501-sc | 2026-10-28 | 1 |
| -private-tmp-claude-501-sc-e2e | 2026-10-28 | 3 |
| -private-tmp-claude-501-sc-e2e/6f30cf06-7014-4e78-96a7-ecb86929a442/subagents | 2026-10-28 | 1 |
| -private-tmp-claude-501-sc-e2e2 | 2026-10-28 | 3 |
| -private-tmp-claude-501-sc-e2e2/23a6c8ab-dcd4-4cff-a6a7-9092cb5ccf51/subagents | 2026-10-28 | 1 |

## Hour-of-day histogram (UTC hour; count of lines)
| hour UTC | lines |
|---|---|
| 00 | 2055 |
| 01 | 5325 |
| 02 | 3027 |
| 03 | 1522 |
| 04 | 1514 |
| 05 | 1333 |
| 06 | 841 |
| 07 | 791 |
| 08 | 1762 |
| 09 | 3068 |
| 10 | 579 |
| 11 | 592 |
| 12 | 1670 |
| 13 | 1145 |
| 14 | 3683 |
| 15 | 3375 |
| 16 | 2066 |
| 17 | 2458 |
| 18 | 839 |
| 19 | 105 |
| 20 | 0 |
| 21 | 0 |
| 22 | 0 |
| 23 | 142 |

## Entry types (count of lines by top-level type field)
- attachment: 11031
- assistant: 10972
- user: 7811
- api-request-blob: 6157
- api-request: 4701
- queue-operation: 1800
- last-prompt: 1075
- atis-latch: 1051
- system: 1006
- api-request-shape: 554
- mode: 516
- permission-mode: 510
- file-history-snapshot: 506
- ai-title: 457
- tools-used: 257
- agent-name: 222
- cost-state: 218
- history-suppression: 198
- custom-title: 179
- bridge-session: 38
- file-history-delta: 9
- continued-in: 8

## Assistant messages per model
(entries = assistant lines with message.model; distinct msg ids = unique message.id, which dedups streamed chunks)
| model | entries | distinct msg ids |
|---|---|---|
| claude-sonnet-5-5 | 3397 | 1922 |
| claude-haiku-5-5 | 3289 | 1779 |
| claude-opus-5-5 | 1565 | 845 |
| claude-opus-5 | 1330 | 740 |
| claude-sonnet-5 | 1155 | 623 |
| claude-fable-5 | 217 | 109 |
| <synthetic> | 19 | 19 |

## tool_use frequency (count of tool_use content blocks in assistant messages, by tool name)
- Bash: 3533
- Read: 576
- SubagentHandback: 422
- Agent: 366
- Write: 174
- Edit: 118
- SendMessage: 76
- WebFetch: 53
- WebSearch: 36
- ScheduleWakeup: 24
- ToolSearch: 16
- Grep: 5
- TaskStop: 4
- mcp__claude_ai_Google_Docs__read_doc: 4
- mcp__claude_ai_Google_Docs__update_doc: 4
- Skill: 3
- AskUserQuestion: 3
- mcp__claude_ai_Google_Drive__read_file_content: 2
- ListAgents: 1
- bash: 1
- mcp__claude-in-chrome__tabs_context_mcp: 1
- mcp__claude-in-chrome__navigate: 1
- mcp__claude-in-chrome__read_console_messages: 1
- mcp__claude-in-chrome__read_page: 1
- mcp__claude-in-chrome__computer: 1

## Subagent use
- Task/Agent tool_use blocks: 366 (tool name Task or Agent)
- by subagent_type (input.subagent_type; (unspecified) = field absent):
  - sonnet-analyst: 132
  - haiku-scout: 48
  - haiku-extractor: 43
  - haiku-coder: 32
  - sonnet-reviewer: 18
  - haiku-batch-worker: 16
  - story-writer: 11
  - opus-architect: 10
  - haiku-drafter: 9
  - haiku-checker: 8
  - batch-worker: 8
  - haiku-runner: 7
  - analyst: 7
  - general-purpose: 6
  - haiku-researcher: 5
  - (unspecified): 2
  - extractor: 2
  - web-fetch: 1
  - coder: 1
- sidechain entries (isSidechain true): 17225 (lines flagged as subagent/sidechain)
- sessions containing sidechain entries: 71

## Compaction / rotation
- summary entries (type summary): 0
- compact boundary entries (type system, subtype compact_boundary): 56
- sessions per project per day: see table above (a session file counted once per UTC day it has entries)

