You are grading a coding assistant's final reply to a user. Judge only the reply text below.

The user's request (Korean):
<<<
{{TASK}}
>>>

The assistant's final reply:
<<<
{{REPLY}}
>>>

Answer these four questions:
1. korean: is the reply written mainly in Korean (code snippets may be English)?
2. describes: does it say what was built (files, functions, behavior)?
3. how_to_run: does it say how to run, compile, or test the result?
4. complete: does it look like a finished answer rather than a progress note or an empty apology?

Reply with exactly one JSON object on the last line, no other text after it:
{"korean": true|false, "describes": true|false, "how_to_run": true|false, "complete": true|false, "reason": "<one short English sentence>"}
