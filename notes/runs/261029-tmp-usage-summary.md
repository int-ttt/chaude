# tmp usage summary (terse)
sources: A = 261029-analysis-usage-patterns-pre1027.md ; B = 261029-analysis-usage-patterns-0901-1031.md
requested-literals-check: "270k" not found in A or B ; "80k" not found ; "3570s" not found
nearest-literals: B §3 "7,355,790ms (약 2.04시간, 171건)" ; B §5 "호출당 cache_read 약 400k 이상" ; B §5 "호출당 약 30~60k"

## cutoff / range
- A §1: cutoff timestamp < 2026-10-27T00:00:00Z (UTC) excluded ; range 2026-09-08 01:20 ~ 2026-10-24 08:54 (UTC) ; active days 18
- B §1: range 2026-09-08T01:20Z ~ 2026-10-29T06:34Z ; 09-01~09-07 and 10-30~31 no records ; split 10-27 onward = 3 days
- B §4: pre-cutoff 09-01~10-26 vs post 10-27~10-31 (actual ~10-29)

## record / file counts
- A §1: records 6,626 (uuid dedup) ; jsonl 537 ; raw lines 49,441 ; excluded post-cutoff 31,396 ; no-timestamp 11,419 ; broken JSON 0
- B §1: jsonl 540 ; raw lines 49,582 ; valid records 38,134 ; sessions 175 ; no-timestamp 11,448 ; parse fail 0
- B §1: metrics records 313 (84 on 10-28 + 229 on 10-29 ; 10-29 is snapshot)
- B §4: pre 6,626 ; post 31,508 ; post share 82.6%
- B §2: daily 10-27 2,357 / 10-28 22,084 / 10-29 7,067 (3 days = 31,508)

## session counts
- A §3: sessions 7 (record-count median 9, max 6,274) ; token stats n=3 (median ~2,200만 ; max ~12.3억)
- B §4: sessions pre 6 ; post 169 ; pre 18 days in 6 sessions
- B §4 note: pre1027 said 7, this run says 6 (sessionId-missing 9 records; method difference, estimated)

## token / usage figures
- A §3 (all included records, usage field sum): input 5,575 ; output 2,554,029 ; cache_read 1,208,570,617 ; cache_creation 42,335,338
- A §3: cache_read ~96.6% of input-side tokens ; cache_creation ~3.3% ; non-cache input ~0
- A §3 by model cache_read: opus-5 ~5.0억 ; sonnet-5 ~4.9억 ; fable-5 ~1.6억 ; sonnet-5-5 ~0.58억
- A §3 output: opus-5 1.22M ; sonnet-5 1.02M ; together ~88%
- B §3 (dedup by message id): output ~4.82M ; cache_read ~811.9M ; cache_creation ~34.0M ; non-cache input ~1.3만
- B §3 dedup cache_read by model: opus-5 284.9M ; sonnet-5 264.2M ; fable-5 82.2M ; sonnet-5-5 78.0M ; opus-5-5 54.2M ; haiku-5-5 48.4M
- B §3 per-call avg cache_read (estimate): opus-5 ~385k ; sonnet-5 ~424k ; fable-5 ~754k ; sonnet-5-5 ~40k ; opus-5-5 ~64k ; haiku-5-5 ~27k
- B §3 metrics tokens (200 result rows): input 642 ; output 178,983 ; cache_creation 1,255,410 ; cache_read 12,255,800
- B §3 processing time: 7,355,790ms (~2.04h, 171 rows)
- B §3 cost: cost_usd sum 82.61 USD (194 numeric, 6 null excluded, lower bound) ; by tag haiku 69.79 / sonnet 3.81 / opus 7.40 ; covers 10-28~29 only
- B §5 pre-10-26 per-call cache_read ~400k+ ; post ~30-60k
- B §5 total cache_read pre 662.8M vs post 149.2M

## record types / models / tools
- A §2 record types: assistant 2,799 ; attachment 1,729 ; user 1,605 ; system 398 ; queue-operation 86 ; file-history-delta 9
- A §2 models (message.model records): opus-5 1,330 ; sonnet-5 1,155 ; fable-5 217 ; sonnet-5-5 79 ; opus-5-5 9 ; synthetic 9
- B §2 models (assistant records / message id): sonnet-5-5 3,414 / 1,932 ; haiku-5-5 3,326 / 1,798 ; opus-5-5 1,565 / 845 ; opus-5 1,330 / 740 ; sonnet-5 1,155 / 623 ; fable-5 217 / 109 ; synthetic 19 ; id total ~6,050
- B §2 router (metrics classify 72): haiku 50 (~69%) ; opus 14 ; sonnet 8 ; turn records with model key: haiku 107, opus 14, sonnet 8
- A §2 tools (tool_use blocks 1,155): Bash 1,069 (~93%) ; ScheduleWakeup 24 ; Edit 18 ; Write 8 ; Read 7 ; ToolSearch 6
- B §2 tools (distinct tool_use id): Bash 3,543 ; Read 583 ; SubagentHandback 428 ; Agent 370 ; Write 178 ; Edit 118 ; SendMessage 76 ; WebFetch 53 ; WebSearch 36 ; ScheduleWakeup 24 ; ToolSearch 16 ; Grep 5

## distribution
- A §2 daily: 09-08 1,349 ; 09-09 985 ; 10-02 1,225 ; 09-24 617 ; 09-10 461 ; 09-11 380 ; 10-24 36
- A §2 hour UTC: 13~15h 2,714 (~41%) ; 02h 668 ; 07h 606 ; 09~11h and 20~22h ~0
- B §2 hour UTC: 01h 5,325 ; 14h 3,683 ; 15h 3,375 ; 09h 3,068 ; 02h 3,027 ; 20~22h 0 ; 19h 105
- A §2 project: VocalAnalyze 6,289 (~95%) / 4 sessions ; UnitV2M12 301 / 2 ; home 36 / 1
- B §2 project: claude-harness 18,216 (47.8%) ; VocalAnalyze 6,289 ; Documents/univ 5,010 ; TextGame 3,654 ; claude-lean 2,748

## settings (current value, not dated)
- A §1 and B §1: effortLevel medium ; permissions.defaultMode auto ; no model key

## doubts
- A §5: 11,419 no-timestamp lines excluded ; session-level stats low confidence (n=3 for tokens) ; possible duplicate usage sums (token totals may be high, unverified)
- B §6: 09-01~09-07 absence unknown (unused vs pruned) ; 11,448 no-timestamp excluded (23%) ; metrics 2 days only ; 10-29 snapshot ; KST conversions are assumptions
