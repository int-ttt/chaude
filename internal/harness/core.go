package harness

import (
	"bufio"
	"bytes"
	"crypto/rand"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/exec"
	"strconv"
	"strings"
	"sync"
	"time"

	"chaude/internal/osenv"
)

type event = map[string]any

// cleanEnv: ANTHROPIC_API_KEY must be absent (with -p a key silently switches billing to API).
func cleanEnv(compactWindow int) []string {
	var env []string
	for _, kv := range os.Environ() {
		if strings.HasPrefix(kv, "ANTHROPIC_API_KEY=") {
			continue
		}
		if compactWindow != 0 && strings.HasPrefix(kv, "CLAUDE_CODE_AUTO_COMPACT_WINDOW=") {
			continue
		}
		env = append(env, kv)
	}
	if compactWindow != 0 { // child-only; no settings file is touched
		env = append(env, "CLAUDE_CODE_AUTO_COMPACT_WINDOW="+strconv.Itoa(compactWindow))
	}
	return env
}

func num(v any) int {
	if f, ok := v.(float64); ok {
		return int(f)
	}
	return 0
}

func asMap(v any) map[string]any {
	m, _ := v.(map[string]any)
	return m
}

func str(v any) string {
	s, _ := v.(string)
	return s
}

// ctxTokens: context-size proxy = input + cache_read + cache_creation of one API call.
func ctxTokens(usage map[string]any) int {
	return num(usage["input_tokens"]) + num(usage["cache_read_input_tokens"]) + num(usage["cache_creation_input_tokens"])
}

// jsonStr is json.dumps(ensure_ascii=False) without trailing newline.
func jsonStr(v any) string {
	var b bytes.Buffer
	enc := json.NewEncoder(&b)
	enc.SetEscapeHTML(false)
	_ = enc.Encode(v)
	return strings.TrimRight(b.String(), "\n")
}

// parseEvent: unparsable lines are kept verbatim as {"type":"_unparsed"}.
func parseEvent(line string) event {
	var ev event
	if err := json.Unmarshal([]byte(line), &ev); err != nil || ev == nil {
		return event{"type": "_unparsed", "raw": line}
	}
	return ev
}

// SessionOpts configures one claude child.
type SessionOpts struct {
	Model         string
	Cwd           string
	Resume        string
	Extra         []string
	ClaudeBin     string
	NoPermission  bool
	CompactWindow int
}

// Session is one `claude -p` child. Send() a user message, then Events() until result.
type Session struct {
	SessionID string
	cmd       *exec.Cmd
	stdin     io.WriteCloser
	lines     chan string // closed at process stdout EOF

	// idle reader: consumes stdout while no turn is running (see StartIdle)
	idleStop  chan struct{}
	idlePause chan chan struct{}
	idleDone  chan struct{}
	mu        sync.Mutex    // guards bgTurn, skip
	bgTurn    bool          // a child-initiated (background) main turn was seen and its result not yet
	held      chan struct{} // pause token taken by Send, released by Events
	skip      int           // results to discard in Events: stray results of background turns
}

func NewSession(o SessionOpts) (*Session, error) {
	if o.Model == "" {
		o.Model = "haiku"
	}
	if o.ClaudeBin == "" {
		o.ClaudeBin = "claude"
	}
	args := []string{"-p", "--input-format", "stream-json", "--output-format", "stream-json",
		"--verbose", "--include-partial-messages", "--model", o.Model}
	if !o.NoPermission {
		args = append(args, "--permission-prompt-tool", "stdio")
	}
	if o.Resume != "" {
		args = append(args, "--resume", o.Resume)
	}
	args = append(args, o.Extra...)
	cmd := exec.Command(osenv.ResolveBin(o.ClaudeBin), args...)
	cmd.Env = cleanEnv(o.CompactWindow)
	cmd.Dir = o.Cwd
	cmd.Stderr = nil
	in, err := cmd.StdinPipe()
	if err != nil {
		return nil, err
	}
	out, err := cmd.StdoutPipe()
	if err != nil {
		return nil, err
	}
	if err := cmd.Start(); err != nil {
		return nil, err
	}
	s := &Session{cmd: cmd, stdin: in, lines: make(chan string, 256)}
	go func() {
		defer close(s.lines)
		r := bufio.NewReaderSize(out, 1<<20)
		for {
			line, err := r.ReadString('\n')
			if t := strings.TrimSpace(line); t != "" {
				s.lines <- t
			}
			if err != nil {
				return
			}
		}
	}()
	return s, nil
}

func (s *Session) write(obj any) error {
	_, err := io.WriteString(s.stdin, jsonStr(obj)+"\n")
	return err
}

func (s *Session) Send(text string) error {
	if s.held == nil {
		s.held = s.pauseIdle() // idle reader stays off stdout until Events has consumed this turn
	}
	s.mu.Lock()
	if s.bgTurn { // the child is mid background-turn: its `result` precedes ours
		s.skip++
		s.bgTurn = false
	}
	s.mu.Unlock()
	return s.write(map[string]any{"type": "user", "message": map[string]any{"role": "user", "content": text}})
}

func (s *Session) RespondPermission(requestID string, allow bool, toolInput map[string]any, message string) error {
	var body map[string]any
	if allow {
		if toolInput == nil {
			toolInput = map[string]any{}
		}
		body = map[string]any{"behavior": "allow", "updatedInput": toolInput}
	} else {
		if message == "" {
			message = "denied by user"
		}
		body = map[string]any{"behavior": "deny", "message": message}
	}
	return s.write(map[string]any{"type": "control_response", "response": map[string]any{
		"subtype": "success", "request_id": requestID, "response": body}})
}

func (s *Session) Interrupt() error {
	return s.write(map[string]any{"type": "control_request", "request_id": newUUID(),
		"request": map[string]any{"subtype": "interrupt"}})
}

// Events calls fn for each parsed event until the turn's `result` (inclusive), process end or timeout.
// If intr fires, returns true (interrupted) immediately.
func (s *Session) Events(timeout time.Duration, intr <-chan struct{}, fn func(event)) (interrupted bool) {
	rel := s.held
	s.held = nil
	if rel == nil {
		rel = s.pauseIdle()
	}
	if rel != nil {
		defer close(rel)
	}
	deadline := time.NewTimer(timeout)
	defer deadline.Stop()
	for {
		select {
		case <-intr:
			return true
		case <-deadline.C:
			return false
		case line, ok := <-s.lines:
			if !ok {
				return false
			}
			ev := parseEvent(line)
			if id := str(ev["session_id"]); id != "" {
				s.SessionID = id
			}
			if ev["type"] == "result" {
				s.mu.Lock()
				stray := s.skip > 0
				if stray {
					s.skip--
				}
				s.mu.Unlock()
				if stray { // late result of background work, not this turn's
					continue
				}
			}
			fn(ev)
			if ev["type"] == "result" {
				return false
			}
		}
	}
}

// StartIdle starts a goroutine that reads stdout whenever no Events call is active, so that
// control_requests and events from background subagents arriving between turns are handled
// (fn) instead of sitting unread. Events pauses it for the duration of a turn.
func (s *Session) StartIdle(fn func(event)) {
	if s.idleStop != nil {
		return
	}
	stop, pause, done := make(chan struct{}), make(chan chan struct{}), make(chan struct{})
	s.idleStop, s.idlePause, s.idleDone = stop, pause, done
	go func() {
		defer close(done)
		for {
			select {
			case <-stop:
				return
			case rel := <-pause:
				select {
				case <-rel:
				case <-stop:
					return
				}
			case line, ok := <-s.lines:
				if !ok {
					return
				}
				ev := parseEvent(line)
				if id := str(ev["session_id"]); id != "" {
					s.SessionID = id
				}
				s.noteIdle(ev)
				fn(ev)
			}
		}
	}()
}

// noteIdle tracks child-initiated main-thread turns: a stray `result` ends one, and a Send
// while one is in flight must skip its result.
func (s *Session) noteIdle(ev event) {
	s.mu.Lock()
	defer s.mu.Unlock()
	switch str(ev["type"]) {
	case "result":
		s.bgTurn = false
	case "assistant", "stream_event":
		if p, ok := ev["parent_tool_use_id"]; !ok || p == nil || p == "" {
			s.bgTurn = true
		}
	}
}

// StopIdle stops the idle reader (before the session is drained or closed). Waits briefly
// if the reader is blocked in a handler.
func (s *Session) StopIdle() {
	if s.idleStop == nil {
		return
	}
	close(s.idleStop)
	select {
	case <-s.idleDone:
	case <-time.After(3 * time.Second):
	}
	s.idleStop = nil
}

// pauseIdle blocks the idle reader; the caller closes the returned channel to resume it.
func (s *Session) pauseIdle() chan struct{} {
	if s.idleStop == nil {
		return nil
	}
	rel := make(chan struct{})
	select {
	case s.idlePause <- rel:
		return rel
	case <-s.idleDone:
		return nil
	}
}

func (s *Session) Close() {
	s.StopIdle()
	_ = s.stdin.Close()
	done := make(chan struct{})
	go func() { _ = s.cmd.Wait(); close(done) }()
	select {
	case <-done:
	case <-time.After(15 * time.Second):
		_ = s.cmd.Process.Kill()
		<-done
	}
}

func newUUID() string {
	b := make([]byte, 16)
	_, _ = rand.Read(b)
	b[6] = b[6]&0x0f | 0x40
	b[8] = b[8]&0x3f | 0x80
	return fmt.Sprintf("%x-%x-%x-%x-%x", b[0:4], b[4:6], b[6:8], b[8:10], b[10:])
}
